use anyhow::{
    anyhow,
    bail,
    Error,
    Result
};
#[cfg(feature="serde")]
use serde::{
    Deserialize,
    Serialize
};
use std::{
    fs::File,
    io::{
	BufReader,
	Read
    },
    path::Path
};

#[derive(Copy,Clone,Debug)]
struct RawHeader {
    #[allow(dead_code)]
    star0:i32,
    #[allow(dead_code)]
    star1:i32,
    starn:i32,
    stnum:i32,
    mprop:bool,
    #[allow(dead_code)]
    nmag:i32,
    nbent:i32
}

/// Generic entry (star) type, the differences between the in-memory
/// and the on-disk representation being defined by the type parameters
#[cfg_attr(feature="serde",derive(Deserialize,Serialize))]
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

/// Equinox-epoch identifier
#[cfg_attr(feature="serde",derive(Deserialize,Serialize))]
#[derive(Copy,Clone,Debug)]
pub enum Equinox {
    /// Besselian
    B1950,

    /// Julian
    J2000
}

/// Enumerates the kind of star IDs a file contains
#[cfg_attr(feature="serde",derive(Deserialize,Serialize))]
#[derive(Copy,Clone,Debug)]
pub enum IdType {
    /// No star identification numbers are present in this file
    None,

    /// See a separate catalog file for getting the identification numbers
    SeeCatalog,

    /// This file includes star ID numbers
    Included
}

/// An entry in a YBSC file
pub type Star = Entry<u32,f32>;

type RawEntry = Entry<f32,i16>;

/// Memory representation of a Yale Bright Star Catalog (YBSC) file
#[cfg_attr(feature="serde",derive(Deserialize,Serialize))]
#[derive(Clone,Debug)]
pub struct Ybsc {
    /// Which equinox-epoch the data refers to
    pub equinox:Equinox,

    /// Which kind of star ID, if any, this catalog contains
    pub id_type:IdType,

    /// If the proper motion values are valid
    pub have_proper_motion:bool,

    /// The entries of the catalog
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

impl<T,U> Entry<T,U> {
    pub fn valid(&self)->bool {
	self.is[0] != ' ' || self.is[1] != ' '
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

impl TryFrom<i32> for IdType {
    type Error = Error;
    
    fn try_from(stnum:i32)->Result<Self> {
	Ok(match stnum {
	    0 => IdType::None,
	    1 => IdType::SeeCatalog,
	    2 => IdType::Included,
	    _ => bail!("Invalid stnum value {}",stnum)
	})
    }
}

impl Ybsc {
    /// Decode a catalog file from a reader
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
		(Equinox::B1950,hdr.starn as usize)
	    };
	let mut stars = Vec::with_capacity(nstar);
	let have_proper_motion = hdr.mprop;
	let id_type : IdType = hdr.stnum.try_into()?;
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

    /// Convenience function for loading a file
    pub fn load<P:AsRef<Path>>(path:P)->Result<Self> {
	let fd = File::open(path)?;
	let br = BufReader::new(fd);
	Ybsc::read_from(br)
    }
}
