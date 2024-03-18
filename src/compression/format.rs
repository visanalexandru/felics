use super::error::DecompressionError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryFrom;
use std::io::{self, Read, Write};

/// Supported color types by the felics compression algorithm.
#[derive(Debug, PartialEq, Eq)]
pub enum ColorType {
    Gray = 0,
    Rgb = 1,
}

impl TryFrom<u8> for ColorType {
    type Error = DecompressionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorType::Gray),
            1 => Ok(ColorType::Rgb),
            _ => Err(DecompressionError::InvalidColorType),
        }
    }
}

/// Supported pixel depths by the felics compression algorithm.
#[derive(Debug, PartialEq, Eq)]
pub enum PixelDepth {
    Eight = 0,
    Sixteen = 1,
}

impl TryFrom<u8> for PixelDepth {
    type Error = DecompressionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PixelDepth::Eight),
            1 => Ok(PixelDepth::Sixteen),
            _ => Err(DecompressionError::InvalidPixelDepth),
        }
    }
}

pub struct Header {
    pub color_type: ColorType,
    pub pixel_depth: PixelDepth,
    pub width: u32,
    pub height: u32,
}

pub fn write_header<T>(header: Header, mut to: T) -> io::Result<()>
where
    T: Write,
{
    to.write_all(b"FLCS")?;
    to.write_u8(header.color_type as u8)?;
    to.write_u8(header.pixel_depth as u8)?;
    to.write_u32::<BigEndian>(header.width)?;
    to.write_u32::<BigEndian>(header.height)?;
    Ok(())
}

pub fn read_header<T>(mut from: T) -> Result<Header, DecompressionError>
where
    T: Read,
{
    let mut magic = vec![0; 4];
    from.read_exact(&mut magic)?;
    if magic != b"FLCS" {
        return Err(DecompressionError::InvalidSignature);
    }

    let color_type = from.read_u8()?.try_into()?;
    let pixel_depth = from.read_u8()?.try_into()?;
    let width = from.read_u32::<BigEndian>()?;
    let height = from.read_u32::<BigEndian>()?;

    Ok(Header {
        color_type,
        pixel_depth,
        width,
        height,
    })
}
