use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage};
use screenshots::Screen;
use serde::Serialize;
use std::io::Cursor;
use windows_sys::Win32::{Foundation::POINT,UI::WindowsAndMessaging::GetCursorPos};

const MAX_EDGE: u32 = 1600;
const MAX_JPEG_BYTES: usize = 2_500_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenObservation { pub jpeg_base64:String, pub width:u32, pub height:u32, pub monitor_x:i32, pub monitor_y:i32 }

pub fn capture_current_monitor() -> Result<ScreenObservation,String> {
    let mut point=POINT{x:0,y:0};unsafe{GetCursorPos(&mut point)};
    let screen=match Screen::from_point(point.x,point.y){Ok(value)=>value,Err(_)=>Screen::all().map_err(|_|"Kai couldn't access the current display.".to_string())?.into_iter().next().ok_or("No display is available.")?};
    let info=screen.display_info;
    let image=screen.capture().map_err(|_|"Kai couldn't capture the current display.".to_string())?;
    let mut dynamic=DynamicImage::ImageRgba8(image);
    if dynamic.width()>MAX_EDGE||dynamic.height()>MAX_EDGE { dynamic=dynamic.resize(MAX_EDGE,MAX_EDGE,FilterType::Triangle); }
    let mut bytes=Cursor::new(Vec::new());JpegEncoder::new_with_quality(&mut bytes,72).encode_image(&dynamic).map_err(|_|"Kai couldn't compress the screen image.".to_string())?;
    let bytes=bytes.into_inner();if bytes.len()>MAX_JPEG_BYTES{return Err("The screen image was too large to send safely.".into())}
    Ok(ScreenObservation{jpeg_base64:STANDARD.encode(bytes),width:dynamic.width(),height:dynamic.height(),monitor_x:info.x,monitor_y:info.y})
}
