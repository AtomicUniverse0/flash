use std::{
    fmt, str,
    thread::{self, JoinHandle},
};

use core_affinity::CoreId;

use crate::error::{UtilError, UtilResult};

#[derive(Clone, Debug, Default)]
pub struct CpuRange {
    cores: Vec<CoreId>,
    curr_idx: usize,
}

impl CpuRange {
    #[allow(clippy::missing_errors_doc)]
    pub fn new() -> UtilResult<Self> {
        Ok(Self {
            cores: core_affinity::get_core_ids().ok_or(UtilError::NoCpuCores)?,
            curr_idx: 0,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn all() -> UtilResult<Self> {
        match core_affinity::get_core_ids() {
            Some(cores) if !cores.is_empty() => Ok(Self { cores, curr_idx: 0 }),
            _ => Err(UtilError::NoCpuCores),
        }
    }

    pub fn reset(&mut self) {
        self.curr_idx = 0;
    }

    pub fn set_affinity_next(&mut self) -> impl FnOnce() -> bool {
        let core_id = self.cores[self.curr_idx];
        self.curr_idx = (self.curr_idx + 1) % self.cores.len();

        move || core_affinity::set_for_current(core_id)
    }

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

impl str::FromStr for CpuRange {
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

impl fmt::Display for CpuRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        let mut i = 0;

        while i < self.cores.len() {
            let start = self.cores[i].id;
            let mut end = start;

            while i + 1 < self.cores.len() && self.cores[i + 1].id == end + 1 {
                end = self.cores[i + 1].id;
                i += 1;
            }

            if start == end {
                parts.push(format!("{start}"));
            } else {
                parts.push(format!("{start}-{end}"));
            }

            i += 1;
        }

        write!(f, "{}", parts.join(","))
    }
}
