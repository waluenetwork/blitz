
use anyhow::Result;
use rustc_hash::FxHashMap;
use std::process::{Child, Command};
use tracing::{debug, error, warn};

#[derive(Debug)]
pub struct TerminalSession {
    pub id: u64,
    pub process: Child,
    pub surface_id: Option<u64>,
    pub command: String,
}

pub struct TerminalManager {
    terminals: FxHashMap<u64, TerminalSession>,
    next_terminal_id: u64,
}

impl TerminalManager {
    pub fn new() -> Self {
        debug!("Creating terminal manager");
        
        Self {
            terminals: FxHashMap::default(),
            next_terminal_id: 1,
        }
    }
    
    pub fn spawn_terminal(&mut self, command: &str) -> Result<u64> {
        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;
        
        debug!("Spawning terminal {} with command: {}", terminal_id, command);
        
        let process = Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn terminal: {}", e))?;
        
        let terminal = TerminalSession {
            id: terminal_id,
            process,
            surface_id: None,
            command: command.to_string(),
        };
        
        self.terminals.insert(terminal_id, terminal);
        
        debug!("Successfully spawned terminal {}", terminal_id);
        Ok(terminal_id)
    }
    
    pub fn kill_terminal(&mut self, terminal_id: u64) -> Result<()> {
        if let Some(mut terminal) = self.terminals.remove(&terminal_id) {
            if let Err(e) = terminal.process.kill() {
                warn!("Failed to kill terminal {} process: {}", terminal_id, e);
            }
            debug!("Killed terminal {}", terminal_id);
        }
        Ok(())
    }
    
    pub fn associate_surface(&mut self, terminal_id: u64, surface_id: u64) -> Result<()> {
        if let Some(terminal) = self.terminals.get_mut(&terminal_id) {
            terminal.surface_id = Some(surface_id);
            debug!("Associated terminal {} with surface {}", terminal_id, surface_id);
        }
        Ok(())
    }
    
    pub fn get_terminal_surface(&self, terminal_id: u64) -> Option<u64> {
        self.terminals.get(&terminal_id)?.surface_id
    }
    
    pub fn list_terminals(&self) -> Vec<u64> {
        self.terminals.keys().copied().collect()
    }
    
    pub fn get_terminal_info(&self, terminal_id: u64) -> Option<(&str, Option<u64>)> {
        let terminal = self.terminals.get(&terminal_id)?;
        Some((&terminal.command, terminal.surface_id))
    }
    
    pub fn cleanup_finished_terminals(&mut self) -> Result<Vec<u64>> {
        let mut finished = Vec::new();
        
        self.terminals.retain(|&id, terminal| {
            match terminal.process.try_wait() {
                Ok(Some(_)) => {
                    debug!("Terminal {} has finished", id);
                    finished.push(id);
                    false
                }
                Ok(None) => true, // Still running
                Err(e) => {
                    error!("Error checking terminal {} status: {}", id, e);
                    true // Keep it for now
                }
            }
        });
        
        Ok(finished)
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
