use anyhow::{
    anyhow,
    bail,
    Error,
    Result
};
use std::io::Read;

#[derive(Copy,Clone,Debug)]
pub struct RawHeader {
    pub star0:i32,
    pub star1:i32,
    pub starn:i32,
    pub stnum:i32,
    pub mprop:bool,
    pub nmag:i32,
    pub nbent:i32
}

#[derive(Copy,Clone,Debug)]
pub struct Entry<T,U> {
    /// Catalog number
    pub xno:T,

    /// B1950 right ascension (radian)
    pub sra0:f64,

    /// B1950 declination (radian)
    pub sdec0:f64,

    /// Spectral type
    pub is:[char;2],

    /// V magnitude
    pub mag:U,
    
    /// Right ascension proper motion (radian/year)
    pub xrpm:f32,

    /// Declination proper motion (radian/year)
    pub xdpm:f32,
}

#[derive(Copy,Clone,Debug)]
pub enum Equinox {
    B1950,
    J2000
}

#[derive(Copy,Clone,Debug)]
pub enum IdType {
    None,
    SeeCatalog,
    Included
}

pub type Star = Entry<u32,f32>;
pub type RawEntry = Entry<f32,i16>;

#[derive(Clone,Debug)]
pub struct Ybsc {
    pub equinox:Equinox,
    pub id_type:IdType,
    pub have_proper_motion:bool,
    pub stars:Vec<Star>
}

fn read_char<R:Read>(mut r:R)->Result<char> {
    let mut x = [0;1];
    r.read_exact(&mut x)?;
    char::from_u32(x[0] as u32)
	.ok_or_else(|| anyhow!("Invalid char {}",x[0]))
}

fn read_i16<R:Read>(mut r:R)->Result<i16> {
    let mut x = [0;2];
    r.read_exact(&mut x)?;
    Ok(i16::from_le_bytes(x))
}

fn read_i32<R:Read>(mut r:R)->Result<i32> {
    let mut x = [0;4];
    r.read_exact(&mut x)?;
    Ok(i32::from_le_bytes(x))
}

fn read_u32<R:Read>(mut r:R)->Result<u32> {
    let mut x = [0;4];
    r.read_exact(&mut x)?;
    Ok(u32::from_le_bytes(x))
}

fn read_u64<R:Read>(mut r:R)->Result<u64> {
    let mut x = [0;8];
    r.read_exact(&mut x)?;
    Ok(u64::from_le_bytes(x))
}

fn read_f32<R:Read>(r:R)->Result<f32> {
    let x = read_u32(r)?;
    Ok(f32::from_bits(x))
}

fn read_f64<R:Read>(r:R)->Result<f64> {
    let x = read_u64(r)?;
    Ok(f64::from_bits(x))
}

impl RawHeader {
    pub fn read_from<R:Read>(mut r:R)->Result<Self> {
	let star0 = read_i32(&mut r)?;
	let star1 = read_i32(&mut r)?;
	let starn = read_i32(&mut r)?;
	let stnum = read_i32(&mut r)?;
	let mprop = read_i32(&mut r)? != 0;
	let nmag = read_i32(&mut r)?;
	let nbent = read_i32(&mut r)?;
	Ok(Self {
	    star0,
	    star1,
	    starn,
	    stnum,
	    mprop,
	    nmag,
	    nbent
	})
    }
}

impl RawEntry {
    pub fn read_from<R:Read>(mut r:R)->Result<Self> {
	let xno = read_f32(&mut r)?;
	let sra0 = read_f64(&mut r)?;
	let sdec0 = read_f64(&mut r)?;
	let is0 = read_char(&mut r)?;
	let is1 = read_char(&mut r)?;
	let is = [is0,is1];
	let mag = read_i16(&mut r)?;
	let xrpm = read_f32(&mut r)?;
	let xdpm = read_f32(&mut r)?;
	Ok(Self {
	    xno,
	    sra0,
	    sdec0,
	    is,
	    mag,
	    xrpm,
	    xdpm
	})
    }

    pub fn valid(&self)->bool {
	self.is[0] != ' ' || self.is[1] != ' '
    }
}

impl TryFrom<RawEntry> for Star {
    type Error = Error;

    fn try_from(raw:RawEntry)->Result<Star> {
	let Entry {
	    xno,
	    sra0,
	    sdec0,
	    is,
	    mag,
	    xrpm,
	    xdpm
	} = raw;
	if !xno.is_finite() || xno < 0.0 {
	    bail!("Invalid star number {}",xno);
	}
	let xno = xno as u32;
	let mag = mag as f32 / 100.0;
	Ok(Self {
	    xno,
	    sra0,
	    sdec0,
	    is,
	    mag,
	    xrpm,
	    xdpm
	})
    }
}

impl Ybsc {
    pub fn read_from<R:Read>(mut r:R)->Result<Self> {
	let hdr = RawHeader::read_from(&mut r)?;
	if hdr.nbent != 32 {
	    bail!("Number of bytes per entry {} is not 32",
		  hdr.nbent);
	}
	let (equinox,nstar) =
	    if hdr.starn < 0 {
		(Equinox::J2000,-hdr.starn as usize)
	    } else {
		(Equinox::B1950,-hdr.starn as usize)
	    };
	let mut stars = Vec::with_capacity(nstar);
	let have_proper_motion = hdr.mprop;
	let id_type = match hdr.stnum {
	    0 => IdType::None,
	    1 => IdType::SeeCatalog,
	    2 => IdType::Included,
	    _ => bail!("Invalid stnum value {}",hdr.stnum)
	};
	for _ in 0..nstar {
	    let entry = RawEntry::read_from(&mut r)?;
	    if entry.valid() {
		let star = Star::try_from(entry)?;
		stars.push(star);
	    }
	}
	Ok(Self {
	    equinox,
	    have_proper_motion,
	    id_type,
	    stars
	})
    }
}
