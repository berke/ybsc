use anyhow::{
    bail,
    Result
};

use ybsc::Ybsc;

fn main()->Result<()> {
    let args : Vec<String> = std::env::args().collect();

    if args.len() != 2 {
	bail!("Specify input");
    }

    let ybsc = Ybsc::load(&args[1])?;
    for star in &ybsc.stars {
	println!("{:8} {:9.3} {:+9.3} {}{} {:+4.2} {:+9.3e} {:+9.3e}",
		 star.xno,
		 star.sra0,
		 star.sdec0,
		 star.is[0],
		 star.is[1],
		 star.mag,
		 star.xrpm,
		 star.xdpm);
    }
    Ok(())
}
