use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub tasks: BTreeMap<u64, Task>,
    pub users: BTreeMap<u64, User>,
}

impl Database {
    pub fn new() -> Self {
        Database {
            tasks: BTreeMap::new(),
            users: BTreeMap::new(),
        }
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string(&self)?;
        let mut file = std::fs::File::create("database.json")?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load_or_create() -> Result<Self, anyhow::Error> {
        if !Path::new("database.json").exists() {
            let database = Database::new();
            database.save()?;
        }
        let json = std::fs::read_to_string("database.json")?;
        let database: Self = serde_json::from_str(&json)?;
        Ok(database)
    }

    pub fn insert_task(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    pub fn update_task(&mut self, id: u64, task: Task) {
        self.tasks.insert(id, task);
    }

    pub fn delete_task(&mut self, id: u64) {
        self.tasks.remove(&id);
    }

    pub fn get_task(&self, id: u64) -> Option<Task> {
        self.tasks.get(&id).cloned()
    }

    pub fn get_tasks(&self) -> Vec<Task> {
        self.tasks.values().cloned().collect()
    }

    pub fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    pub fn update_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    pub fn delete_user(&mut self, id: u64) {
        self.users.remove(&id);
    }

    pub fn get_user(&self, id: u64) -> Option<User> {
        self.users.get(&id).cloned()
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users
            .values()
            .find(|user| user.username == username)
            .cloned()
    }

    pub fn get_users(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }
}
