use std::env;
use sqlite3::State;

pub struct Database {
}

pub struct History {
    pub user_id: f64,
    pub link: String,
}

impl Database {
    /// Adds a new story to the database
    ///
    /// # Arguments
    ///
    /// * 'user_id' - Telegram user ID
    /// * 'link' - Link to the site
    pub fn add_history(user_id: f64, link: &str) -> State {
        // Adding a new row to the database
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let connection = sqlite3::open(database_url).expect("Failed to connect to the database");
        let mut db = connection.prepare("INSERT INTO history VALUES (?, ?)").unwrap();

        // The numbers 1 and 2 denote the location of the question mark in the query
        db.bind(1, user_id.to_string().as_str()).unwrap();
        db.bind(2, link).unwrap();

        // Save the changes to the database
        db.next().unwrap()
    }

    /// Checks if a history exists that has the same reference
    ///
    /// # Arguments
    ///
    /// * 'user_id' - Telegram user ID
    /// * 'link' - Link to the site
    pub fn is_history_exists(user_id: f64, link: &str) -> bool {
        // We get the history list and check if there are any items in it
        let vec: Vec<History> = Database::get_histories(user_id, Option::from(link));
        return vec.iter().count() > 0
    }

    /// Retrieves histories of recent user requests
    ///
    /// # Arguments
    ///
    /// * 'user_id' - Telegram user ID
    /// * 'link' - Link to the site if you need to prevent duplicate links
    pub fn get_histories(user_id: f64, link: Option<&str>) -> Vec<History> {
        let query: &str;

        // Depending on whether the reference is None, type in your query
        if let Some(_) = link { query = "SELECT * FROM history WHERE user_id = ? AND link = ?" }
        else { query = "SELECT * FROM history WHERE user_id = ?" }

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let connection = sqlite3::open(database_url).expect("Failed to connect to the database");

        let mut db = connection.prepare(query).unwrap();
        db.bind(1, user_id.to_string().as_str()).unwrap();

        // If there is a reference, bind the second value
        if let Some(str) = link {
            db.bind(2, str).unwrap();
        }

        // List
        let mut vec: Vec<History> = Vec::new();

        // Get the rows and add a new history to the list
        while let State::Row = db.next().unwrap() {
            vec.push(History {
                user_id: db.read::<f64>(0).unwrap(),
                link: db.read::<String>(1).unwrap()
            })
        }

        return vec
    }

    /// Clears the user's recent request history
    ///
    /// # Arguments
    ///
    /// * 'user_id' - Telegram user ID
    pub fn clear_histories(user_id: f64) -> State {
        // Specify in the request that we want to delete all histories in which the user ID matches the required one
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let connection = sqlite3::open(database_url).expect("Failed to connect to the database");
        let mut db = connection.prepare("DELETE FROM history WHERE user_id = ?").unwrap();

        db.bind(1, user_id.to_string().as_str()).unwrap();


        // Also, don't forget to save the changes
        db.next().unwrap()
    }
}

#[cfg(test)]
mod database_test {
    use super::*;

    // This is where you indicate your path
    static PATH: &str = "C:\\Users\\Dmitry\\RustroverProjects\\N_i_Kit_OS\\databases\\main_database.sqlite";

    #[test]
    fn test_connection_database() {
        let _database: Database = Database::new(PATH);

        assert!(true)
    }

    #[test]
    fn test_insert_into_database() {
        let mut database: Database = Database::new(PATH);

        database.add_history(654352f64, "Hello world!");
        database.add_history(654352f64, "No");
        database.add_history(654352f64, "Yes");
        database.add_history(654352f64, "Ggg");
        database.add_history(3552f64, "Lol");
        database.add_history(3552f64, "Go");

        assert!(true)
    }

    #[test]
    fn test_is_history_exists() {
        let database: Database = Database::new(PATH);

        let bool1 = database.is_history_exists(654352f64, "Ggg");
        let bool2 = database.is_history_exists(654352f64, "Gg");

        println!("Is exist: {}", bool1);
        println!("Is exist: {}", bool2);

        assert!(true)
    }

    #[test]
    fn test_get_histories() {
        let database: Database = Database::new(PATH);

        let vec = database.get_histories(654352f64, None);

        for history in vec {
            println!("User ID: {} | Link: {}", history.user_id, history.link);
        }

        assert!(true)
    }

    #[test]
    fn test_clear_history() {
        let mut database: Database = Database::new(PATH);

        database.clear_histories(654352f64);
        database.clear_histories(3552f64);

        println!("Histories for {} is cleared!", 654352f64);
        println!("Histories for {} is cleared!", 3552f64);

        assert!(true)
    }
}