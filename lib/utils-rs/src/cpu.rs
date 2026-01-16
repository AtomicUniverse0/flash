use std::{
    str::FromStr,
    thread::{self, JoinHandle},
};

use core_affinity::CoreId;

use crate::error::{UtilError, UtilResult};

#[derive(Clone, Debug)]
pub struct CpuRing {
    cores: Vec<CoreId>,
    curr_idx: usize,
}

impl CpuRing {
    #[allow(clippy::missing_errors_doc)]
    pub fn new() -> UtilResult<Self> {
        Ok(Self {
            cores: core_affinity::get_core_ids().ok_or(UtilError::NoCpuCores)?,
            curr_idx: 0,
        })
    }

    pub fn reset(&mut self) {
        self.curr_idx = 0;
    }

    pub fn set_affinity_next(&mut self) -> impl FnOnce() -> bool {
        let core_id = self.cores[self.curr_idx];
        self.curr_idx = (self.curr_idx + 1) % self.cores.len();

        move || core_affinity::set_for_current(core_id)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn spawn<F>(&mut self, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.cores.is_empty() {
            thread::spawn(f)
        } else {
            let core_id = self.cores[self.curr_idx];
            self.curr_idx = (self.curr_idx + 1) % self.cores.len();

            thread::spawn(move || {
                core_affinity::set_for_current(core_id);
                f();
            })
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn spawn_multiple<F>(&mut self, funcs: impl IntoIterator<Item = F>) -> Vec<JoinHandle<()>>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.cores.is_empty() {
            funcs.into_iter().map(|f| thread::spawn(f)).collect()
        } else {
            funcs
                .into_iter()
                .map(|f| {
                    let core_id = self.cores[self.curr_idx];
                    self.curr_idx = (self.curr_idx + 1) % self.cores.len();

                    thread::spawn(move || {
                        core_affinity::set_for_current(core_id);
                        f();
                    })
                })
                .collect()
        }
    }
}

impl FromStr for CpuRing {
    type Err = UtilError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let available_cores = match core_affinity::get_core_ids() {
            Some(cores) if !cores.is_empty() => cores,
            _ => return Err(UtilError::NoCpuCores),
        };

        let mut cores = Vec::new();
        for part in s.split(',') {
            if let Some((start, end)) = part.split_once('-')
                && let Ok(start) = start.trim().parse::<usize>()
                && let Ok(end) = end.trim().parse::<usize>()
                && start <= end
            {
                for core in start..=end {
                    if let Some(core_id) = available_cores.iter().find(|c| c.id == core) {
                        cores.push(*core_id);
                    } else {
                        return Err(UtilError::CpuCoreNotFound(core));
                    }
                }
            } else if let Ok(core) = part.trim().parse::<usize>() {
                if let Some(core_id) = available_cores.iter().find(|c| c.id == core) {
                    cores.push(*core_id);
                } else {
                    return Err(UtilError::CpuCoreNotFound(core));
                }
            } else {
                return Err(UtilError::InvalidCpuCoreRange(part.to_string()));
            }
        }

        Ok(Self { cores, curr_idx: 0 })
    }
}
