use process_memory::{DataMember, Memory, TryIntoProcessHandle};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

type Pid = i32;

#[derive(Clone)]
pub struct ProcessHandle {
    pid: Pid,
    handle: process_memory::ProcessHandle,
    proc_maps: Vec<proc_maps::MapRange>,
}

impl ProcessHandle {
    pub fn new(pid: Pid) -> Self {
        Self {
            pid,
            handle: (pid as process_memory::Pid)
                .try_into_process_handle()
                .unwrap(),
            proc_maps: proc_maps::get_process_maps(pid).unwrap(),
        }
    }

    pub fn get_pid(&self) -> i32 {
        self.pid
    }

    pub fn get_proc_maps(&self) -> &Vec<proc_maps::MapRange> {
        &self.proc_maps
    }

    pub fn read_mem<T: Clone + Copy>(&self, offset: usize) -> Result<T> {
        let member = DataMember::<T>::new_offset(self.handle, vec![offset]);
        Ok(unsafe { member.read() }?)
    }

    pub fn write_mem<T: Clone + Copy>(&self, value: T, offset: usize) -> Result<()> {
        let member = DataMember::<T>::new_offset(self.handle, vec![offset]);
        Ok(member.write(&value)?)
    }
}

pub enum ScanType {
    Initialize,
    Prune,
}

pub trait MemoryScanner {
    /// scan process memory for a value given a compile-type data type
    fn find_values<T: Clone + Copy + Sync>(
        &mut self,
        value: &T,
        condition: impl Fn(&T, &T) -> bool + Sync,
        scan_type: ScanType,
    ) -> Result<&Vec<usize>>;

    /// search for pointers in memory which are referencing a given address
    fn find_pointers(&self, address: usize) -> Result<Vec<usize>>;
}

pub struct DefaultScanner {
    /// process handle to read memory
    process: ProcessHandle,
    /// possible locations which have the target value
    candidates: Vec<usize>,
}

impl DefaultScanner {
    pub fn new(process: ProcessHandle) -> Self {
        Self {
            process,
            candidates: Default::default(),
        }
    }
}

impl MemoryScanner for DefaultScanner {
    fn find_values<T: Clone + Copy + Sync>(
        &mut self,
        value: &T,
        cond: impl Fn(&T, &T) -> bool + Sync,
        scan_type: ScanType,
    ) -> Result<&Vec<usize>> {
        match scan_type {
            ScanType::Initialize => {
                self.candidates = Default::default();

                let (sender, receiver) = std::sync::mpsc::channel();

                // TODO: improve the speed of this method
                self.process
                    .get_proc_maps()
                    .into_par_iter()
                    .filter(|map| map.is_read() && map.is_write() && map.filename().is_none())
                    .for_each_with(sender, |s, map| {
                        for offset in (0..map.size()).map(|i| i + map.start()) {
                            if let Ok(copied) = self.process.read_mem(offset) {
                                if cond(value, &copied) {
                                    s.send(offset).unwrap();
                                }
                            }
                        }
                    });

                self.candidates = receiver.into_iter().collect();
            }
            ScanType::Prune => {
                self.candidates.retain(|address| {
                    self.process
                        .read_mem(*address)
                        .map_or(false, |copied| cond(value, &copied))
                });
            }
        }

        Ok(&self.candidates)
    }

    fn find_pointers(&self, address: usize) -> Result<Vec<usize>> {
        let mut pointers = Vec::new();

        for map in self
            .process
            .get_proc_maps()
            .iter()
            .filter(|map| map.is_read() && map.is_write() && map.filename().is_none())
        {
            for offset in (0..map.size()).map(|i| i + map.start()) {
                if let Ok(copied) = self.process.read_mem(offset) {
                    if address == copied {
                        pointers.push(offset);
                    }
                }
            }
        }

        Ok(pointers)
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
