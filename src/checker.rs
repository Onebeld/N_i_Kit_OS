pub struct Checker {
    pub user_id: u64,
    pub is_activated: bool,
    pub links: Vec<String>
}

impl Clone for Checker {
    fn clone(&self) -> Self {
        Checker {
            user_id: self.user_id,
            is_activated: self.is_activated,
            links: self.links.clone()
        }
    }
}

impl Checker {
    pub fn new(user_id: u64, links: Vec<String>) -> Checker {
        Checker {
            user_id,
            is_activated: true,
            links
        }
    }

    pub fn check_websites(&mut self) {
        println!("{} called!", self.user_id)
    }
}