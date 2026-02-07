pub struct BotRegistry {
    bots: Vec<String>,
}

impl BotRegistry {
    pub fn new() -> Self {
        BotRegistry { bots: vec![] }
    }

    pub fn register(&mut self, name: String) {
        self.bots.push(name);
    }

    pub fn bots(&self) -> Vec<&str> {
        self.bots.iter().map(|bot| bot.as_str()).collect()
    }
}
