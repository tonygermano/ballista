use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::sync::Mutex;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use home::env::Env;
use home::env::OS_ENV;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionEntry {
    address: String,
    #[serde(rename="heapSize")]
    heap_size: String,
    icon: String,
    id: String,
    #[serde(rename="javaHome")]
    java_home: String,
    name: String,
    username: Option<String>,
    password: Option<String>
}

pub struct ConnectionStore {
    cache: Mutex<HashMap<String, ConnectionEntry>>,
    location: PathBuf
}

impl ConnectionStore {
    pub fn init(location: PathBuf) -> Result<Self, Error> {
        let mut f = File::open(&location);
        if let Err(e) = f {
            f = File::create(&location);
        }

        let f = f?;

        let mut cache = HashMap::new();
        let data : serde_json::Result<HashMap<String, ConnectionEntry>> = serde_json::from_reader(f);
        if let Ok(data) = data {
            cache = data;
        }

        Ok(ConnectionStore{location, cache: Mutex::new(cache)})
    }

    pub fn to_json_array_string(&self) -> String {
        let mut sb = String::with_capacity(1024);
        let len = self.cache.lock().unwrap().len();
        sb.push('[');
        for (pos, ce) in self.cache.lock().unwrap().values().enumerate() {
            let c = serde_json::to_string(ce).expect("failed to serialize ConnectionEntry");
            sb.push_str(c.as_str());
            if pos + 1 < len {
                sb.push(',');
            }
        }
        sb.push(']');

        sb
    }

    pub fn save(&self, mut ce: ConnectionEntry) -> Result<(), Error> {
        if ce.id.is_empty() {
            ce.id = uuid::Uuid::new_v4().to_string();
        }

        let mut jh = ce.java_home.trim().to_string();
        if jh.is_empty() {
            jh = find_java_home();
        }
        ce.java_home = jh;

        self.cache.lock().unwrap().insert(ce.id.clone(), ce);
        self.flush_to_disk()?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), Error> {
        self.cache.lock().unwrap().remove(id);
        self.flush_to_disk()?;
        Ok(())
    }

    pub fn import(&self, file_path: &str) -> Result<String, Error> {
        let f = File::open(file_path)?;
        let data: Vec<ConnectionEntry> = serde_json::from_reader(f)?;
        let mut count = 0;
        let java_home = find_java_home();
        for mut ce in data {
            ce.java_home = java_home.clone();
            self.cache.lock().unwrap().insert(ce.id.clone(), ce);
            count = count + 1;
        }

        self.flush_to_disk()?;
        Ok(format!("imported {} connections", count))
    }

    fn flush_to_disk(&self) -> Result<(), Error> {
        let val = serde_json::to_string(&self.cache)?;
        let mut f = OpenOptions::new().write(true).open(&self.location);
        if let Err(e) = f {
            println!("unable to open file for writing: {}", e.to_string());
            return Err(Error::new(e));
        }
        let r = f.unwrap().write_all(val.as_bytes())?;
        Ok(())
    }
}

pub fn find_java_home() -> String {
    let mut java_home = String::from("");
    if let Some(jh) = OS_ENV.var_os("JAVA_HOME") {
        java_home = String::from(jh.to_str().unwrap());
        println!("JAVA_HOME is set to {}", java_home);
    }

    if java_home.is_empty() {
        let out = Command::new("/usr/libexec/java_home").args(["-v", "1.8"]).output();
        if let Ok(out) = out {
            if out.status.success() {
                java_home = String::from_utf8(out.stdout).expect("failed to create UTF-8 string from OsStr");
                println!("/usr/libexec/java_home -v 1.8 returned {}", java_home);
            }
        }
    }
    java_home
}