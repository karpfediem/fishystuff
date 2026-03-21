use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};

use crate::config::AppConfig;
use crate::store::{DoltMySqlStore, Store};

pub type SharedState = Arc<AppState>;

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

pub struct AppState {
    pub config: AppConfig,
    pub store: Arc<dyn Store>,
    pub cache: CacheStore,
}

impl AppState {
    pub fn new(config: AppConfig) -> Result<SharedState> {
        let store = Arc::new(
            DoltMySqlStore::new(config.database_url.clone(), config.defaults.clone())
                .map_err(|err| anyhow!(err.0.message.clone()))?,
        );
        let cache = CacheStore::new(
            config.cache_zone_stats_max,
            config.cache_effort_max,
            config.cache_log,
        );
        Ok(Arc::new(Self {
            config,
            store,
            cache,
        }))
    }

    #[cfg(test)]
    pub fn for_tests(config: AppConfig, store: Arc<dyn Store>) -> SharedState {
        let cache = CacheStore::new(
            config.cache_zone_stats_max,
            config.cache_effort_max,
            config.cache_log,
        );
        Arc::new(Self {
            config,
            store,
            cache,
        })
    }
}

pub struct CacheStore {
    pub zone_stats: Mutex<BoundedCache>,
    pub effort_grid: Mutex<BoundedCache>,
}

impl CacheStore {
    fn new(zone_stats_max: usize, effort_max: usize, log: bool) -> Self {
        Self {
            zone_stats: Mutex::new(BoundedCache::new("zone_stats", zone_stats_max, log)),
            effort_grid: Mutex::new(BoundedCache::new("effort_grid", effort_max, log)),
        }
    }
}

pub struct BoundedCache {
    name: &'static str,
    max_entries: usize,
    log: bool,
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

impl BoundedCache {
    fn new(name: &'static str, max_entries: usize, log: bool) -> Self {
        Self {
            name,
            max_entries,
            log,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        if self.map.contains_key(key) {
            self.touch(key);
            if self.log {
                eprintln!("[cache:{}] hit {}", self.name, key);
            }
            return self.map.get(key).cloned();
        }
        if self.log {
            eprintln!("[cache:{}] miss {}", self.name, key);
        }
        None
    }

    pub fn insert(&mut self, key: String, value: String) {
        if self.max_entries == 0 {
            return;
        }
        self.map.insert(key.clone(), value);
        self.touch(&key);
        self.evict_if_needed();
    }

    fn touch(&mut self, key: &str) {
        self.order.retain(|existing| existing != key);
        self.order.push_back(key.to_string());
    }

    fn evict_if_needed(&mut self) {
        while self.map.len() > self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
                if self.log {
                    eprintln!("[cache:{}] evict {}", self.name, oldest);
                }
            } else {
                break;
            }
        }
    }
}
