use clap::Parser;
use proc_maps::get_process_maps;
use process_memory::{DataMember, Memory, Pid, TryIntoProcessHandle};

#[derive(Parser)]
struct Cli {
    process_id: usize,
    #[clap(short)]
    target_value: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let maps = get_process_maps(args.process_id as i32)?;

    for map in maps.iter().filter(|s| s.is_read() && s.is_exec()) {
        let size = map.size();
        let start = map.start();

        let handle = (args.process_id as Pid).try_into_process_handle().unwrap();
        for member in
            (0..size).map(|offset| DataMember::<u32>::new_offset(handle, vec![offset + start]))
        {
            let new = unsafe { member.read()? };
            if new == args.target_value {
                println!("{}", member.get_offset()?);
            }
        }
    }
    Ok(())
}

trait UpdateRule {}
