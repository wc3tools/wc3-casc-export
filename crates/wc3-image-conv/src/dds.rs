use anyhow::Result;
use image::dds::DdsDecoder;
use image::jpeg::JPEGEncoder;
use image::png::PNGEncoder;
use image::ImageDecoder;
use std::io::Write;

pub fn encode_png<W>(bytes: &[u8], w: W) -> Result<()>
where
  W: Write,
{
  let dec = DdsDecoder::new(bytes)?;
  let (width, height) = dec.dimensions();
  // println!("width = {}, height = {}", width, height);
  let color_type = dec.color_type();
  // println!("color_type = {:?}", color_type);
  // println!("bytes = {}", dec.total_bytes());
  let mut dec_bytes = vec![0; dec.total_bytes() as usize];
  dec.read_image(&mut dec_bytes)?;
  let enc = PNGEncoder::new(w);
  enc.encode(&dec_bytes, width, height, color_type)?;
  Ok(())
}

pub fn encode_jpg<W>(bytes: &[u8], mut w: W) -> Result<()>
where
  W: Write,
{
  let dec = DdsDecoder::new(bytes)?;
  let (width, height) = dec.dimensions();
  // println!("width = {}, height = {}", width, height);
  let color_type = dec.color_type();
  // println!("color_type = {:?}", color_type);
  // println!("bytes = {}", dec.total_bytes());
  let mut dec_bytes = vec![0; dec.total_bytes() as usize];
  dec.read_image(&mut dec_bytes)?;
  let mut enc = JPEGEncoder::new(&mut w);
  enc.encode(&dec_bytes, width, height, color_type)?;
  Ok(())
}

#[test]
fn test_encode_png() {
  let f = std::fs::File::create("sample/btnsylvanas.png").unwrap();
  encode_png(&*include_bytes!("../sample/btnsylvanas.dds"), f).unwrap();
}
