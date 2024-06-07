use std::collections::BTreeMap;
use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    id: u64,
    name: String,
    description: String,
    completed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    tasks: BTreeMap<u64, Task>,
    users: BTreeMap<u64, User>,
}

impl Database {
    fn new() -> Self {
        Database {
            tasks: BTreeMap::new(),
            users: BTreeMap::new(),
        }
    }

    fn save(&self) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string(&self)?;
        let mut file = std::fs::File::create("database.json")?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn load() -> Result<Self, anyhow::Error> {
        let json = std::fs::read_to_string("database.json")?;
        let database: Self = serde_json::from_str(&json)?;
        Ok(database)
    }

    fn insert_task(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    fn update_task(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    fn delete_task(&mut self, id: u64) {
        self.tasks.remove(&id);
    }

    fn get_task(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }

    fn get_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn update_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn delete_user(&mut self, id: u64) {
        self.users.remove(&id);
    }

    fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }

    fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.users.values().find(|user| user.username == username)
    }

    fn get_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Hello, world!");

    Ok(())
}
