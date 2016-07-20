#[derive(Debug)]
#[derive(Clone)]
pub struct Client {
    pub pid: u32,
    pub hwnd: u32, // I don't seem to be able to use winapi::windef::HWND as type directly here.
    pub name: String,
    pub collect: bool,
    pub offx: i32, // X offset for collecting purposes
    pub offy: i32, // Y offset for collecting purposes
}

#[derive(Debug)]
pub struct Setup {
    pub timeout: u64,
    pub keysel: u64,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Config {
    pub base: BaseConfig,
    pub collector: CollectorConfig,
    pub kitebot: KitebotConfig,
    pub systembuffer: SystembufferConfig,
    pub dungeons: DungeonsConfig,
    pub dungeon: Vec<Dungeon>
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Accounts {
    pub account: Vec<Account>
}

impl Default for Config {
    fn default() -> Config {
        Config {
            base: BaseConfig::default(),
            collector: CollectorConfig::default(),
            kitebot: KitebotConfig::default(),
            systembuffer: SystembufferConfig::default(),
            dungeons: DungeonsConfig::default(),
            dungeon: vec![Dungeon::default()]
        }
    }
}

impl Default for Accounts {
    fn default() -> Accounts {
        Accounts {
            account: vec![Account::default()]
        }
    }
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Account {
    pub account: String,
    pub name: String,
    pub collect: bool
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct BaseConfig {
    pub executable: String,
    pub offset: String
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct CollectorConfig {
    pub timeout: u64,
    pub modifier: u64
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct KitebotConfig {
    pub timeout: u64,
    pub keytime: u64
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct SystembufferConfig {
    pub offset: String
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct DungeonsConfig {
    pub offset: String
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Dungeon {
    pub name: String,
    pub coordinates: Vec<[f32; 3]>
}

impl Default for Dungeon {
    fn default() -> Dungeon {
        Dungeon {
            name: "default".to_owned(),
            coordinates: vec![[0.0, 0.0, 0.0]]
        }
    }
}

impl Default for Account {
    fn default() -> Account {
        Account {
            name: "default".to_owned(),
            account: "default".to_owned(),
            collect: false
        }
    }
}

impl Default for BaseConfig {
    fn default() -> BaseConfig {
        BaseConfig {
            executable: "Neuz.exe".to_owned(),
            offset: "0".to_owned()
        }
    }
}

impl Default for CollectorConfig {
    fn default() -> CollectorConfig {
        CollectorConfig {
            timeout: 1800u64,
            modifier: 1u64
        }
    }
}

impl Default for KitebotConfig {
 fn default() -> KitebotConfig {
        KitebotConfig {
            timeout: 600u64,
            keytime: 200u64
        }
    }
}

impl Default for SystembufferConfig {
    fn default() -> SystembufferConfig {
        SystembufferConfig {
            offset: "0".to_owned()
        }
    }
}

impl Default for DungeonsConfig {
    fn default() -> DungeonsConfig {
        DungeonsConfig{
            offset: "0".to_owned()
        }
    }
}
