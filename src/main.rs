#![allow(non_snake_case)]
extern crate winapi;
extern crate stb_tt_sys;
extern crate tensorflow_sys_tools;
extern crate multiinput;
//extern crate miniz; for use at some future date



use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::iter::once;
use std::io::Error;
use std::mem;
use std::io::prelude::*;
use std::fs::{File, create_dir, read_dir};
use std::time;
use std::ptr::{null, null_mut};
use std::thread::sleep;
use winapi::shared::windef::{HWND, RECT, HDC, HWND__, HDC__};
use winapi::um::wingdi::{BITMAP, BITMAPINFO, BITMAPINFOHEADER, SRCCOPY, RGBQUAD};
use winapi::um::wingdi as gdi32;
use gdi32::CreateCompatibleDC;
use winapi::um::winuser as user32;
use winapi::um::libloaderapi as kernel32;
use winapi::um::xinput;
use stb_tt_sys::*;
use tensorflow_sys_tools::tensorflow_tools::*;
use tensorflow_sys_tools::tensorflow_bindings::tensorflow_init;



//AUDIO  CAPTURE STUFF
use winapi::um::combaseapi::{CoCreateInstance, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{CLSID_MMDeviceEnumerator, IMMDeviceEnumerator, eRender, eConsole, IMMDevice};
use winapi::um::audioclient::{IID_IAudioClient, IAudioClient, IID_IAudioCaptureClient, IAudioCaptureClient, AUDCLNT_BUFFERFLAGS_SILENT};
use winapi::um::audiosessiontypes::{AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_SHAREMODE_SHARED};
use winapi::shared::guiddef::GUID;
use winapi::shared::mmreg::WAVEFORMATEX;
use winapi::Interface;
use winapi::um::dsound;
use winapi::um::dsound::IDirectSound;

use multiinput::RawInputManager;



const C4_YELLOW :[f32;4] = [1.0, 1.0, 0.0, 1.0];
const C4_WHITE  :[f32;4] = [1.0, 1.0, 1.0, 1.0];
const C4_BLUE   :[f32;4] = [0.0, 0.0, 1.0, 1.0];
const C4_BLACK  :[f32;4] = [0.0, 0.0, 0.0, 1.0];
const C4_GREY   :[f32;4] = [0.1, 0.1, 0.1, 1.0];
const C4_RED    :[f32;4] = [1.0, 0.0, 0.0, 1.0];
const C4_GREEN  :[f32;4] = [0.0, 1.0, 0.0, 1.0];

/*
100 images per run
    + 3.034
    + 2.46
    + 2.567
    + 2.99
    + 2.90

AVG: 2.79s
time per frame: 0.0279
for reference 1/60th of a seconds is 0.0166

writing an image seems to be about  0.007s which is about 1/4th of the time.
So we can see 50 frames per second without writing the frames out
*/

//TODO
// + I want to be able to change fonts
// + fdraw functions where position data is floating point
// + copy chunks of pixel buffers where you can instead of iterating

//TODO: ai related
// prgram crashes when player jumps is a particular x range


//TODO: screen_capture related
// + test and use app
// + debug selecting gamepad vs keyboard
//    + hard to get out of gamepad key selection mode if you never select any thing
// + make keyboard and gamepad button down events good I get too many signals for one button down
// + work through TODOs
// FUTURE TODO
// +  make bmp scaling better
// + change view port width and height to match the aspect ratio of the game window
// + clean up how we select where to place text cursor...works but is currently janky -- feb 22, 2019
// + cheat engine like tool <= I don't think i need this we can just call windows to get
//    relevent info
//https://github.com/fenix01/cheatengine-library

pub mod windowslayer{
use winapi::shared::windef::{HWND, RECT, HDC, HWND__, HDC__};
use std::ptr::{null, null_mut};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt; //For encode wide
    pub struct WindowHandleDC{
        pub window_handle : *mut HWND__,
        pub window_dc     : *mut HDC__,
    }
    pub fn load_handle_dc(window_name: &str, )->WindowHandleDC{ unsafe{
        use std::iter::once;
        use user32::{FindWindowW, GetWindowDC};

        let windows_string: Vec<u16> = OsStr::new(window_name).encode_wide().chain(once(0)).collect();

        let handle = FindWindowW(null_mut(), windows_string.as_ptr());
        let handle_dc = WindowHandleDC{ window_handle: handle,
                        window_dc: GetWindowDC(handle)};

        return handle_dc;
    }}
}

pub mod renderingtools{
extern crate stb_tt_sys;

use std::ptr::{null, null_mut};
use winapi::um::wingdi::{BITMAP, BITMAPINFO, BITMAPINFOHEADER, SRCCOPY, RGBQUAD};
use stb_tt_sys::*;
use std::io::prelude::*;
use std::fs::File;

    pub static mut GLOBAL_FONTINFO : stbtt_fontinfo = new_stbtt_fontinfo();

    pub struct WindowsCanvas{
        pub info : BITMAPINFO,
        pub w: i32,
        pub h: i32,
        pub buffer: *mut std::ffi::c_void
    }
    #[derive(Debug)]
    #[derive(Default, Clone)]
    pub struct TGBitmapHeaderInfo{
        pub header_size:        u32,
        pub width:              i32,
        pub height:             i32,
        pub planes:             u16,
        pub bit_per_pixel:      u16,
        pub compression:        u32,
        pub image_size:         u32,
        pub x_px_per_meter:     i32,
        pub y_px_per_meter:     i32,
        pub colors_used:        u32,
        pub colors_important:   u32,
    }


    #[repr(packed)]
    #[derive(Clone)]
    pub struct TGBitmapFileHeader{
       pub  type_:              u16,
       pub  size_:              u32,
       pub  reserved_1:         u16,
       pub  reserved_2:         u16,
       pub  off_bits:           u32,
    }


    #[derive(Clone)]
    pub struct TGBitmap{
       pub file_header:        TGBitmapFileHeader,
       pub info_header:        TGBitmapHeaderInfo,
       pub rgba:               Vec<u8>,

      //For ease of use
       pub width  : i32,
       pub height : i32,
    }

    pub fn loadBMP(filename: &str)->TGBitmap{unsafe{
        let mut rt = TGBitmap::new(0,0);
        let mut f = File::open(filename).expect("BMP file could not be opened.");
        let mut img_buffer = Vec::new();
        f.read_to_end(&mut img_buffer).expect("Buffer could not be read.");

        let it =  img_buffer.as_ptr() as *const u8;
        rt.file_header.type_ =  *it.offset(0) as u16;// == 0x42;
        rt.file_header.type_ = (*it.offset(1) as u16) << 2;// == 0x4d;
        rt.file_header.size_ = *(it.offset(2) as *const u32);
        rt.file_header.reserved_1 = *(it.offset(6) as *const u16);
        rt.file_header.reserved_2 = *(it.offset(8) as *const u16);
        rt.file_header.off_bits =  *(it.offset(10) as *const u32);


        rt.info_header.header_size = *(it.offset(14) as *const u32);
        rt.info_header.width       = *(it.offset(18) as *const i32);
        rt.info_header.height      =  *(it.offset(22) as *const i32);
        rt.info_header.planes      =  *(it.offset(26) as *const u16);
        rt.info_header.bit_per_pixel = *(it.offset(28) as *const u16);
        rt.info_header.compression = *(it.offset(30) as *const u32);
        rt.info_header.image_size  = *(it.offset(34) as *const u32);
        rt.info_header.x_px_per_meter = *(it.offset(38) as *const i32);
        rt.info_header.y_px_per_meter = *(it.offset(42) as *const i32);
        rt.info_header.colors_used  = *(it.offset(46) as *const u32);
        rt.info_header.colors_important = *(it.offset(50) as *const u32);


        let buffer = img_buffer[rt.file_header.off_bits as usize ..].to_vec();
        rt.rgba = buffer;
        rt.width =  rt.info_header.width; 
        rt.height =  rt.info_header.height; 

        return rt;
    }}

    impl TGBitmap{
        pub fn new(w: i32, h: i32)->TGBitmap{
            TGBitmap{
                file_header: TGBitmapFileHeader{
                    type_: 0x4d42, //BM
                    size_: 0,
                    reserved_1: 0,
                    reserved_2: 0,
                    off_bits: 0,
                },
                info_header:   TGBitmapHeaderInfo{
                        header_size:        0,
                        width:              w,
                        height:             h,
                        planes:             1,
                        bit_per_pixel:      32,
                        compression:        0,
                        image_size:         0,
                        x_px_per_meter:     0,
                        y_px_per_meter:     0,
                        colors_used:        0,
                        colors_important:   0,
                },
                rgba: vec![0;4 * (w*h) as usize],
                width  : w,
                height : h,
            }

        }
    }

    pub fn renderDefaultToBuffer( canvas: &mut WindowsCanvas, default_color: Option<[u8;4]>){unsafe{
        let buffer = canvas.buffer as *mut u32;
        let w = canvas.w;
        let h = canvas.h;

        let mut r = 100;
        let mut g = 50;
        let mut b = 50;
        match default_color{
            Some(arr) =>{
                r = arr[0] as u32;
                g = arr[1] as u32;
                b = arr[2] as u32;
            },
            None =>{
            }
        }
        //TODO speedup
        for i in 0..(w*h) as isize {
            *buffer.offset(i) = 0x00000000 + (r << 16) +  (g << 8)  + b;
        }
    }}
    pub fn resizeBMP(source_bmp: &TGBitmap, w: i32, h: i32)->TGBitmap{unsafe{
        let mut bmp = TGBitmap::new(w, h);
        {
            if source_bmp.info_header.width < w{
                println!("Trash", );
            }
            if source_bmp.info_header.height < h{
                println!("Trash");
            }
            let scale_w = w as f32 / source_bmp.info_header.width as f32;
            let scale_h = h as f32 / source_bmp.info_header.height as f32;



            let source_buffer = source_bmp.rgba.as_ptr();
            let dst_buffer = bmp.rgba.as_mut_ptr() as *mut u32;

            let bytes_per_pix = (source_bmp.info_header.bit_per_pixel / 8) as isize;

            for j in 0..source_bmp.info_header.height{
                for i in 0..source_bmp.info_header.width{
                    let mut _i;
                    let mut _j;
                    _i = (i as f32 * scale_w) as i32;
                    _j = (j as f32 * scale_h) as i32;


                    if _i >= w { _i = w-1; }
                    if _j >= h { _j = h-1; }


                    let src_rgb = source_buffer.offset(  bytes_per_pix * (i + source_bmp.info_header.width * j) as isize);
                    let src_r =  *(src_rgb as *const u8).offset(2);
                    let src_g =  *(src_rgb as *const u8).offset(1);
                    let src_b =  *(src_rgb as *const u8).offset(0);

                    let mut _scale_w = scale_w;
                    let mut _scale_h = scale_h;
                    fn get_correct_scale_for_pixel(original_index: i32, scale: f32)->f32{
                        let mut post_index  = scale * (original_index as f32);
                        let mut _it = post_index;
                        if ((post_index - post_index.trunc()) / scale).trunc() >= 1.0{
                            _it -= 1.0 * ((post_index - post_index.trunc()) / scale).trunc() * scale;
                        }
                        return  1.0/ (  (((1.0+_it).trunc() - _it ) / scale).trunc() + 1.0) ;
                    }
                    _scale_h = get_correct_scale_for_pixel(j, scale_h);
                    _scale_w = get_correct_scale_for_pixel(i, scale_w);
                    ///////////////////////////////

                    let r = (src_r as f32 * _scale_w * _scale_h) as u32;
                    let g = (src_g as f32 * _scale_w * _scale_h) as u32;
                    let b = (src_b as f32 * _scale_w * _scale_h) as u32;

                    *dst_buffer.offset( (_i + w * _j) as isize ) += 0x00000000 + (r << 16) + (g << 8) + b;
                }
            }
        }
        return bmp;
    }}



    pub fn drawBMP( canvas: &mut WindowsCanvas, source_bmp: &TGBitmap, x: i32, y: i32, alpha: f32,
                _w: Option<i32>, _h: Option<i32>){unsafe{

        if alpha < 0.0 {
            println!("A negative alpha as passed to drawBMP");
            return;
        }
        let w;
        let h;

        match _w {
            Some(int) => w = int,
            None => w = source_bmp.info_header.width,
        }
        match _h {
            Some(int) => h = int,
            None => h = source_bmp.info_header.height,
        }

        let bmp = if w == source_bmp.info_header.width &&
                     h == source_bmp.info_header.height{
                         (*source_bmp).clone()
                     } else {
                         resizeBMP(source_bmp, w, h)
                     };

        {   //render bmp_buffer to main_buffer

            let buffer = canvas.buffer as *mut u32;
            let gwidth = canvas.w as i32;
            let gheight = canvas.h as i32;
            let offset = (x + y * gwidth) as i32;
            let bit_stride = (bmp.info_header.bit_per_pixel / 8) as i32;

            let color = bmp.rgba.as_ptr();
            for i in (0..bmp.info_header.height).rev(){
                //TODO
                //when alpha is one copy the bmp bits instead of iterating through the array
                if alpha < 0.99 {
                    for j in 0..bmp.info_header.width{

                        if (j + i*gwidth + offset) < 0 {continue;}
                        if (j + i*gwidth + offset) > gwidth * gheight {continue;}

                        if j + x > gwidth {continue;}
                        if i + y > gheight {continue;}


                        let r = (*color.offset(( bit_stride * (j + i * bmp.info_header.width) + 2) as isize) as f32 * alpha ) as u32;
                        let g = (*color.offset(( bit_stride * (j + i * bmp.info_header.width) + 1) as isize) as f32 * alpha ) as u32;
                        let b = (*color.offset(( bit_stride * (j + i * bmp.info_header.width) + 0) as isize) as f32 * alpha ) as u32;


                        let dst_rgb = buffer.offset( (j + i*gwidth + offset) as isize);
                        let _r = (*(dst_rgb as *const u8).offset(2) as f32 * (1.0 - alpha )) as u32;
                        let _g = (*(dst_rgb as *const u8).offset(1) as f32 * (1.0 - alpha )) as u32;
                        let _b = (*(dst_rgb as *const u8).offset(0) as f32 * (1.0 - alpha )) as u32;

                        let r_cmp = (r+_r).min(255).max(0);
                        let g_cmp = (g+_g).min(255).max(0);
                        let b_cmp = (b+_b).min(255).max(0);

                        *buffer.offset( (j + i*gwidth + offset) as isize) = 0x00000000 + (r_cmp << 16) + (g_cmp << 8) + b_cmp;
                    }
                }
                else{
                    let _w = bmp.info_header.width as usize;
                    let _off_src = i as isize * _w as isize * bit_stride as isize;
                    let _off_dst = i as isize * gwidth as isize;
                    std::ptr::copy::<u32>(color.offset(_off_src) as *const u32, buffer.offset( _off_dst + offset as isize), _w);
                }
            }
        }
    }}

    pub fn drawChar( canvas: &mut WindowsCanvas, character: char, x: i32, y: i32,
                 color: [f32; 4], size: f32 )->i32{unsafe{

        //Check that globalfontinfo has been set
        if GLOBAL_FONTINFO.data == null_mut() {
            println!("Global font has not been set.");
            return -1;
        }
        //let mut now = time::Instant::now();

        //construct a char buffer
        let mut char_buffer;
        let cwidth;
        let cheight;
        let scale;
        {//NOTE
         //this accounts for about 10% of character rendering time.
         //If we want an easy speed up we can save the results to a global buffer  map
         // can only add to it when there is a new character being renedered
         // however if we build in release mode it doesn't really matter
            let mut x0 = 0i32;
            let mut x1 = 0i32;
            let mut y0 = 0i32;
            let mut y1 = 0i32;
            let mut ascent = 0;
            let mut descent = 0;

            stbtt_GetFontVMetrics(&mut GLOBAL_FONTINFO as *mut stbtt_fontinfo,
                                  &mut ascent as *mut i32,
                                  &mut descent as *mut i32, null_mut());
            scale = stbtt_ScaleForPixelHeight(&GLOBAL_FONTINFO as *const stbtt_fontinfo, size);
            let baseline = (ascent as f32 * scale ) as i32;

            cwidth = (scale * (ascent - descent) as f32 ) as usize + 4;
            cheight = (scale * (ascent - descent) as f32 ) as usize + 4;
            char_buffer = vec![0u8; cwidth * cheight];

            //render char to buffer
            stbtt_GetCodepointBitmapBoxSubpixel(&GLOBAL_FONTINFO as *const stbtt_fontinfo, character as u8, scale, scale, 0.0,0.0,
                                                &mut x0 as *mut i32,
                                                &mut y0 as *mut i32,
                                                &mut x1 as *mut i32,
                                                &mut y1 as *mut i32);
            stbtt_MakeCodepointBitmapSubpixel(  &GLOBAL_FONTINFO as *const stbtt_fontinfo,
                                                &mut char_buffer[cwidth*(baseline + y0) as usize + (5 + x0) as usize ] as *mut u8,
                                                 x1-x0+2, y1-y0, cwidth as i32, scale, scale,0.0, 0.0, character as i32);
        }
        //println!("time to render to buffer {:?} {} {:x} {}", now.elapsed(), character, character as i32, size);
        //now = time::Instant::now();

        //NOTE
        //If the character is invisible then don't render
        if character as u8 > 0x20{   //render char_buffer to main_buffer
            let buffer = canvas.buffer as *mut u32;
            let gwidth = canvas.w as usize;
            let gheight = canvas.h as usize;
            let offset = (x as usize + y as usize * gwidth) as usize;
            for i in 0..cheight{
                for j in 0..cwidth{

                    if (j + i*gwidth + offset) > gwidth * gheight {continue;}

                    if j + x as usize  > gwidth {continue;}
                    if i + y as usize  > gheight {continue;}

                    let text_alpha = char_buffer[j + cwidth * (cheight - 1 - i)] as f32;
                    let a = color[3];
                    let r = (color[0] * text_alpha * a) as u32;
                    let g = (color[1] * text_alpha * a) as u32;
                    let b = (color[2] * text_alpha * a) as u32;

                    let dst_rgb = buffer.offset( (j + i*gwidth + offset) as isize);
                    let _r = (*(dst_rgb as *const u8).offset(2) as f32 * (255.0 - text_alpha * a )/255.0 ) as u32;
                    let _g = (*(dst_rgb as *const u8).offset(1) as f32 * (255.0 - text_alpha * a )/255.0 ) as u32;
                    let _b = (*(dst_rgb as *const u8).offset(0) as f32 * (255.0 - text_alpha * a )/255.0 ) as u32;

                    *buffer.offset( (j + i*gwidth + offset) as isize) = 0x00000000 + (r+_r << 16) + (g+_g << 8) + b+_b;
                }
            }
        }
        //println!("time to render to screen {:?}", now.elapsed());

        let mut adv : i32 = 0;
        let mut lft_br : i32 = 0; // NOTE: Maybe remove this
        stbtt_GetCodepointHMetrics(&GLOBAL_FONTINFO as *const stbtt_fontinfo, character as i32, &mut adv as *mut i32, &mut lft_br as *mut i32);
        return (adv as f32 * scale) as i32;
    }}

    pub fn drawString( canvas: &mut WindowsCanvas, string: &str, x: i32, y: i32,
                 color: [f32; 4], size: f32 )->i32{
        let mut offset = 0;
        for it in string.chars(){
            offset += drawChar(canvas, it, x + offset, y, color, size);
        }
        return offset;
    }

    pub fn drawRect( canvas: &mut WindowsCanvas, rect: [i32; 4], color: [f32; 4], filled: bool ){unsafe{
        //TODO
        //use std::ptr::copy when the alpha component is near or equal to one.
        //This is an optimization

        let buffer = canvas.buffer as *mut u32;

        let c_w = canvas.w as isize;
        let c_h = canvas.h as isize;

        let x = rect[0] as isize;
        let y = rect[1] as isize;
        let w = rect[2] as isize;
        let h = rect[3] as isize;

        let a = color[3];
        let r = (color[0] * a * 255.0) as u32;
        let g = (color[1] * a * 255.0) as u32;
        let b = (color[2] * a * 255.0) as u32;

        let mut fast_rgba_buffer = vec![0x00000000 + (r << 16) +  (g << 8)  + b; w as usize];


        let mut _continue = false;
        if x >= 0 && y >= 0 && x < c_w && y < c_h{
            _continue = true;
        }
        if _continue == false {return;}

        for _j in y..y+h{
            let j = _j as isize;
            if a < 0.99 || filled == false{
                for _i in x..x+w{
                    let i = _i as isize;
                    if i > c_w || j > c_h{
                        continue;
                    }
                    let dst_rgb = buffer.offset( (i + c_w*j) as isize);
                    let _r = (*(dst_rgb as *const u8).offset(2) as f32 * (1.0 - a)) as u32;
                    let _g = (*(dst_rgb as *const u8).offset(1) as f32 * (1.0 - a)) as u32;
                    let _b = (*(dst_rgb as *const u8).offset(0) as f32 * (1.0 - a)) as u32;

                    if filled == false{
                         if (_i - x) > 1 && (_i - x ) < w-2 &&
                            (_j - y) > 1 && (_j - y ) < h-2{continue;}

                         *buffer.offset(i + c_w*j) = 0x00000000 + (r+_r << 16) +  (g+_g << 8)  + b+_b;

                    } else {
                        *buffer.offset(i + c_w*j) = 0x00000000 + (r+_r << 16) +  (g+_g << 8)  + b+_b;
                    }
                }
            }
            else {
                std::ptr::copy::<u32>(fast_rgba_buffer.as_ptr(), buffer.offset(c_w*j + x), w as usize);
            }
        }
    }}

}

use windowslayer::*;
use renderingtools::*;

static mut GLOBAL_BACKBUFFER : WindowsCanvas = WindowsCanvas{
    info : BITMAPINFO{
        bmiHeader : BITMAPINFOHEADER{
            biSize : 0,
            biWidth : 0,
            biHeight : 0,
            biPlanes : 0,
            biBitCount : 0,
            biCompression : 0,//BI_RGB,
            biSizeImage : 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [ RGBQUAD{
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0, }],
    },
    w : 0,
    h : 0,
    buffer : null_mut(),
};
//TODO set this up so that one does not use
//static mut GLOBAL_FONTINFO : stbtt_fontinfo = new_stbtt_fontinfo();
static mut GLOBAL_WINDOWINFO : WindowInfo = WindowInfo{ x: 0, y: 0, w: 0, h: 0};


fn new_rgbquad()->RGBQUAD{
     RGBQUAD{
        rgbBlue: 0,
        rgbGreen: 0,
        rgbRed: 0,
        rgbReserved: 0,
     }
}

fn screen_shot(handle_dc: &WindowHandleDC, number_of_shots: i32, file_prepend: &str, directory_prepend: &str, save_files: bool)->Vec<TGBitmap>{unsafe{
    use gdi32::{CreateCompatibleBitmap, SelectObject, BitBlt, GetObjectW, GetDIBits};

    let mut rt = Vec::new();

    let mut rect: RECT = RECT{ left: 0, top: 0, right: 0, bottom: 0};
    if user32::GetWindowRect(handle_dc.window_handle, &mut rect) != 0{
    } else {
        println!("Coud not get window rect");
        return rt;
    }

    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    let bitmap_handle = CreateCompatibleBitmap( handle_dc.window_dc, w, h);

    if bitmap_handle == null_mut(){
        println!("bitmap was bad.");
        return Vec::new();
    }

    let compat_dc = CreateCompatibleDC(handle_dc.window_dc);
    let mut _capture_count = 0;
    loop {
        let esc = user32::GetAsyncKeyState(0x1B);
        if esc != 0 {
            println!("escape");
            break;
        }

        if _capture_count == number_of_shots{
            break;
        }
        _capture_count = _capture_count + 1;
        if _capture_count > 1 {
            std::thread::sleep(time::Duration::from_millis(10));
        }

        let oldBitmap = SelectObject(compat_dc, bitmap_handle as winapi::shared::windef::HGDIOBJ);
        if BitBlt(compat_dc as HDC, 0, 0, w, h, handle_dc.window_dc as HDC, 0, 0, SRCCOPY) == 0 {
            println!("BitBlt broke {:?}", line!());
        }

        //https://stackoverflow.com/questions/3291167/how-can-i-take-a-screenshot-in-a-windows-application
        //https://msdn.microsoft.com/en-us/library/windows/desktop/dd183402(v=vs.85).aspx
        //https://stackoverflow.com/questions/31302185/rust-ffi-casting-to-void-pointer
        let mut pixels = vec![0u8; (4*w*h) as usize];
        let mut bitmap = BITMAP{bmType: 0, bmWidth: 0, bmHeight: 0, bmWidthBytes: 0, bmPlanes: 0, bmBitsPixel: 0, bmBits: &mut pixels[0] as *mut u8 as *mut winapi::ctypes::c_void};

        GetObjectW(bitmap_handle as *mut winapi::ctypes::c_void, mem::size_of::<BITMAP>() as i32 , &mut bitmap as *mut BITMAP as *mut winapi::ctypes::c_void);

        let mut bitmap_info = BITMAPINFO{
            bmiHeader : BITMAPINFOHEADER{
                biSize : mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth : bitmap.bmWidth,
                biHeight : bitmap.bmHeight,
                biPlanes : 1,
                biBitCount : bitmap.bmBitsPixel,
                biCompression : 0,//BI_RGB,
                biSizeImage : ((w as u32 * bitmap.bmBitsPixel as u32 + 31) / 32) * 4 * h as u32,
                biXPelsPerMeter: 1,
                biYPelsPerMeter: 1,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [new_rgbquad()],
        };

        GetDIBits(handle_dc.window_dc, bitmap_handle, 0, bitmap.bmHeight as u32, &mut pixels[0] as *mut u8 as *mut winapi::ctypes::c_void, &mut bitmap_info as *mut BITMAPINFO, 0);
        SelectObject(compat_dc, oldBitmap);



        let header =  TGBitmapFileHeader{   type_: 0x4d42, //BM
                                                size_:(mem::size_of::<TGBitmapFileHeader>() + mem::size_of::<TGBitmapHeaderInfo>() + 4 * pixels.len()) as u32,
                                                reserved_1: 0,
                                                reserved_2: 0,
                                                off_bits: (mem::size_of::<TGBitmapFileHeader>() + mem::size_of::<TGBitmapHeaderInfo>()) as u32};

        //Redunant please fix
        let info = TGBitmapHeaderInfo{
            header_size:        mem::size_of::<TGBitmapHeaderInfo>() as u32,
            width:              bitmap.bmWidth,
            height:             bitmap.bmHeight,
            planes:             1,
            bit_per_pixel:      bitmap.bmBitsPixel,
            compression:        0,
            image_size:         bitmap_info.bmiHeader.biSizeImage,
            x_px_per_meter:     1,
            y_px_per_meter:     1,
            colors_used:        0,
            colors_important:   0,
        };

        rt.push(TGBitmap{file_header: header, info_header: info, rgba: pixels, width: bitmap.bmWidth, height: bitmap.bmHeight});

    }
    if save_files{for (i, it) in rt.iter().enumerate(){

        let filename = format!("{}/{}_{:}.bmp",directory_prepend, file_prepend, i+1);
        println!("writing {}", filename);
        let mut filebuffer = match File::create(filename){
            Ok(_fb) => _fb,
            Err(_s) => {
                println!("BMP file could not be made. {}", _s);
                break;
            }
        };

        {
            filebuffer.write( &transmute(&it.file_header.type_) ).expect("BMP file_header.type could not be written.");
            filebuffer.write( &transmute(&it.file_header.size_) ).expect("BMP file_header.size could not be written.");
            filebuffer.write( &transmute(&it.file_header.reserved_1) ).expect("BMP file_header.reserverd_1 could not be written.");
            filebuffer.write( &transmute(&it.file_header.reserved_2) ).expect("BMP file_header.reserved_2 could not be written.");
            filebuffer.write( &transmute(&it.file_header.off_bits) ).expect("BMP file_header.off_bits could not be written.");
        }
        {

            filebuffer.write( &transmute(&it.info_header.header_size) ).expect("BMP info_header.header_size could not be written.");
            filebuffer.write( &transmute(&it.info_header.width) ).expect("BMP info_header.width could not be written.");
            filebuffer.write( &transmute(&it.info_header.height) ).expect("BMP info_header.height could not be written.");
            filebuffer.write( &transmute(&it.info_header.planes) ).expect("BMP info_header.planes could not be written.");
            filebuffer.write( &transmute(&it.info_header.bit_per_pixel) ).expect("BMP info_header.bit_per_pixel could not be written.");
            filebuffer.write( &transmute(&it.info_header.compression) ).expect("BMP info_header.compression could not be written.");
            filebuffer.write( &transmute(&it.info_header.image_size) ).expect("BMP info_header.image_size could not be written.");
            filebuffer.write( &transmute(&it.info_header.x_px_per_meter) ).expect("BMP info_header.x_px_per_meter could not be written.");
            filebuffer.write( &transmute(&it.info_header.y_px_per_meter) ).expect("BMP info_header.y_px_per_meter could not be written.");
            filebuffer.write( &transmute(&it.info_header.colors_used) ).expect("BMP info_header.colors_used could not be written.");
            filebuffer.write( &transmute(&it.info_header.colors_important) ).expect("BMP info_header.colors_important could not be written.");
        }
        filebuffer.write( &it.rgba ).expect("BMP rgba arr could not be written.");
    }}
    gdi32::DeleteDC(compat_dc as HDC);
    gdi32::DeleteDC(bitmap_handle as HDC);
    return rt;
}}

fn transmute<T>(t:&T)->Vec<u8>{unsafe{
    let ptr = t as *const T as *const u8;
    let mut v = vec![];
    for i in 0..mem::size_of::<T>(){
        v.push(*ptr.offset(i as isize));
    }
    v
}}



fn getAdvance(character: char, size: f32)->i32{unsafe{
    if GLOBAL_FONTINFO.data == null_mut() {
        println!("Global font has not been set.");
        return -1;
    }
    let mut adv = 0;
    let scale = stbtt_ScaleForPixelHeight(&GLOBAL_FONTINFO as *const stbtt_fontinfo, size);
    stbtt_GetCodepointHMetrics(&GLOBAL_FONTINFO as *const stbtt_fontinfo, character as i32, &mut adv as *mut i32, null_mut());
    return (adv as f32 * scale) as i32;
}}


fn resize_drawsection( canvas: &mut WindowsCanvas, w: i32, h: i32){unsafe{
    use winapi::um::memoryapi::{VirtualAlloc, VirtualFree};
    use winapi::um::winnt::{MEM_COMMIT, PAGE_READWRITE, MEM_RELEASE};

    if canvas.buffer != null_mut(){
        VirtualFree(canvas.buffer as *mut winapi::ctypes::c_void, 0, MEM_RELEASE);
    }
    canvas.info = BITMAPINFO{
        bmiHeader : BITMAPINFOHEADER{
            biSize : mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth : w,
            biHeight : h,
            biPlanes : 1,
            biBitCount : 32,
            biCompression : 0,//BI_RGB,
            biSizeImage : 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [new_rgbquad()],
    };
    canvas.w = w;
    canvas.h = h;

    canvas.buffer = VirtualAlloc(null_mut(), (w*h*32) as usize, MEM_COMMIT, PAGE_READWRITE) as *mut std::ffi::c_void;
    renderDefaultToBuffer( &mut GLOBAL_BACKBUFFER, None);
}}


fn update_window(device_context: HDC, canvas: &WindowsCanvas, x: i32, y: i32, w: i32, h: i32 ){unsafe{
    use gdi32::StretchDIBits;
    let _w = canvas.w;
    let _h = canvas.h;
    StretchDIBits(device_context, 0, 0, w, h, 0, 0, _w, _h, canvas.buffer as *const winapi::ctypes::c_void, &canvas.info as *const BITMAPINFO, 0, SRCCOPY);
}}


extern "system" fn window_callback(window: HWND, message: u32, w_param: usize, l_param: isize )->isize{unsafe{
    use user32::{DefWindowProcA, BeginPaint, EndPaint, PostQuitMessage, GetClientRect};
    use winapi::um::winuser::{WM_SIZE, WM_DESTROY, WM_CLOSE, WM_ACTIVATEAPP, WM_PAINT, PAINTSTRUCT};


    let mut rt = 0;
    match message{
        WM_SIZE=>{
            let mut rect: RECT = RECT{ left: 0, top: 0, right: 0, bottom: 0};
            GetClientRect(window, &mut rect as *mut RECT);
            resize_drawsection(&mut GLOBAL_BACKBUFFER, rect.right - rect.left, rect.bottom - rect.top);
        },
        WM_DESTROY=>{
            PostQuitMessage(0);
        },
        WM_CLOSE=>{
            PostQuitMessage(0);
        },
        WM_ACTIVATEAPP=>{
        },
        WM_PAINT=>{
            let mut rect: RECT = RECT{ left: 0, top: 0, right: 0, bottom: 0};
            let mut canvas = PAINTSTRUCT{hdc: null_mut(), fErase: 0 , rcPaint:rect, fRestore: 0, fIncUpdate: 0, rgbReserved: [0;32]};
            BeginPaint(window, &mut canvas as *mut PAINTSTRUCT );
            {//TODO
             //will soon become my DrawRect function
                let x = canvas.rcPaint.left;
                let y = canvas.rcPaint.top;
                let w = canvas.rcPaint.right - canvas.rcPaint.left;
                let h = canvas.rcPaint.bottom - canvas.rcPaint.top;
                update_window(canvas.hdc, &GLOBAL_BACKBUFFER, x, y, w, h);
            }
            EndPaint(window, &mut canvas as *mut PAINTSTRUCT);
        },
        _=>{
            rt = DefWindowProcA(window, message, w_param, l_param);
        },
    }
    return rt;
}}
fn make_window(){unsafe{
    //TODO temp
    let mut manager = RawInputManager::new().unwrap();
    manager.register_devices(multiinput::DeviceType::Joysticks(multiinput::XInputInclude::False));
    //


    use user32::{RegisterClassW, CreateWindowExW, TranslateMessage, DispatchMessageW, PeekMessageW, LoadCursorW};
    use winapi::um::winuser::{ WNDCLASSW, CW_USEDEFAULT, WS_OVERLAPPEDWINDOW, WS_VISIBLE, MSG, IDC_ARROW};
    use winapi::shared::windef::POINT;

    let instance = kernel32::GetModuleHandleW(null());
    let commandline = winapi::um::processenv::GetCommandLineA();

    let mut mouseinfo = MouseInfo::new();
    let mut joystickinfo : JoystickInfo = Default::default();
    let mut app_data = AppData::new();
    let time_instance = time::Instant::now();

    //NOTE
    //We are missing cmd show. Not sure if we will event need it....

    //https://docs.microsoft.com/en-us/windows/desktop/winmsg/window-class-styles
    let windows_string: Vec<u16> = OsStr::new("HandmadeWindowClass").encode_wide().chain(once(0)).collect();
    let windowclass = WNDCLASSW{style: 0x0020u32 | 0x0001u32 | 0x0002u32,
            lpfnWndProc: Some(window_callback),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: windows_string.as_ptr()};

    if RegisterClassW(&windowclass as *const WNDCLASSW) != 0 {
        let windows_string: Vec<u16> = OsStr::new("Handmade Window").encode_wide().chain(once(0)).collect();
        //TODO
        //Might want to make this alpha
        //https://docs.microsoft.com/en-us/windows/desktop/api/winuser/nf-winuser-createwindowexa
        let window_handle = CreateWindowExW(
                          0 ,
                          windowclass.lpszClassName,
                          windows_string.as_ptr(),
                          WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                          CW_USEDEFAULT,
                          CW_USEDEFAULT,
                          1000,//CW_USEDEFAULT,
                          550,//CW_USEDEFAULT,
                          null_mut(),
                          null_mut(),
                          instance,
                          null_mut());
        if window_handle != null_mut(){
            //TODO
            //This is a standard practice that we will not be using in the future
            //--- I don't know what this is referring to might want to check out later
            let mut nframes = 0;
            'a : loop {
                let mut message = MSG{ hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time: 0, pt: POINT{x:0, y:0} };
                let mut textinfo = TextInfo{character: '\0', timing:0};
                let mut keyboardinfo = KeyboardInfo{key: Vec::new(), status:Vec::new()};

                //NOTE
                //I'm not sure this should be here perhaps some where else...
                mouseinfo.old_lbutton = mouseinfo.lbutton.clone();
                mouseinfo.wheel_delta = 0;
                while PeekMessageW(&mut message as *mut MSG, null_mut(), 0, 0, 0x0001) > 0{
                    {//NOTE: Handle mouse events
                        //Convert to the correct coordinates
                        mouseinfo.x = message.pt.x - GLOBAL_WINDOWINFO.x - 10;
                        mouseinfo.y = GLOBAL_WINDOWINFO.h - ( message.pt.y - GLOBAL_WINDOWINFO.y) - 10;

                        if message.message == winapi::um::winuser::WM_LBUTTONDOWN{ mouseinfo.lbutton = ButtonStatus::Down; }
                        else if message.message == winapi::um::winuser::WM_LBUTTONUP{ mouseinfo.lbutton = ButtonStatus::Up; }
                        //else { mouseinfo.lbutton = ButtonStatus::Default; }//Not sure what this fixed but keep an eye on this

                        //Mouse Wheel stuffs
                        if message.message == winapi::um::winuser::WM_MOUSEWHEEL{
                            let delta_wheel = winapi::um::winuser::GET_WHEEL_DELTA_WPARAM(message.wParam) as i16;
                            mouseinfo.wheel += delta_wheel as isize /120;
                            mouseinfo.wheel_delta = delta_wheel as i32 /120;
                        }
                        else{
                        }

                    }
                    {//Handle text events
                        if message.message == winapi::um::winuser::WM_CHAR{
                            //NOTE
                            //This only handles ascii characters
                            textinfo.character = message.wParam as u8 as char;
                        }else{
                            textinfo.character = '\0';
                        }
                    }
                    {//Handle keyboard events
                        if message.message == winapi::um::winuser::WM_KEYDOWN{
                            if message.wParam == winapi::um::winuser::VK_LEFT as usize{
                                keyboardinfo.key.push(KeyboardEnum::Leftarrow);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            }
                            else if message.wParam == winapi::um::winuser::VK_RIGHT as usize{
                                keyboardinfo.key.push(KeyboardEnum::Rightarrow);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            }
                            else if message.wParam == winapi::um::winuser::VK_UP as usize{
                                keyboardinfo.key.push(KeyboardEnum::Uparrow);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            }
                            else if message.wParam == winapi::um::winuser::VK_DOWN as usize{
                                keyboardinfo.key.push(KeyboardEnum::Downarrow);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            }
                            else if message.wParam == winapi::um::winuser::VK_RETURN as usize{
                                keyboardinfo.key.push(KeyboardEnum::Enter);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            } else {
                                keyboardinfo.key.push(KeyboardEnum::Default);
                                keyboardinfo.status.push(ButtonStatus::Down);
                            }
                        }
                        if message.message == winapi::um::winuser::WM_KEYUP{
                            if message.wParam == winapi::um::winuser::VK_LEFT as usize{
                                keyboardinfo.key.push(KeyboardEnum::Leftarrow);
                                keyboardinfo.status.push(ButtonStatus::Up);
                            }
                            else if message.wParam == winapi::um::winuser::VK_RIGHT as usize{
                                keyboardinfo.key.push(KeyboardEnum::Rightarrow);
                                keyboardinfo.status.push(ButtonStatus::Up);
                            }
                            else if message.wParam == winapi::um::winuser::VK_UP as usize{
                                keyboardinfo.key.push(KeyboardEnum::Uparrow);
                                keyboardinfo.status.push(ButtonStatus::Up);
                            }
                            else if message.wParam == winapi::um::winuser::VK_DOWN as usize{
                                keyboardinfo.key.push(KeyboardEnum::Downarrow);
                                keyboardinfo.status.push(ButtonStatus::Up);
                            } else {
                                keyboardinfo.key.push(KeyboardEnum::Default);
                                keyboardinfo.status.push(ButtonStatus::Up);
                            }

                        }
                    }
                    if message.message == winapi::um::winuser::WM_QUIT{
                        break 'a;
                    }
                    else if message.message == winapi::um::winuser::WM_KEYDOWN && message.wParam == winapi::um::winuser::VK_ESCAPE as usize{
                        break 'a;
                    }
                    TranslateMessage(&mut message as *mut MSG);
                    DispatchMessageW(&mut message as *mut MSG);
                }

                //NOTE
                //The joystick setup is particular to my personal Hori
                if let Some(event) = manager.get_event(){
                    match event{
                        multiinput::event::RawEvent::JoystickButtonEvent(_usize_id, _1usize, _State)=>{ 
                            use multiinput::event::State;

                            match _State{
                                //TODO Double check me
                                State::Pressed => {
                                    if _1usize == 0 { joystickinfo.x = ButtonStatus::Down; }
                                    if _1usize == 1 { joystickinfo.a = ButtonStatus::Down; }
                                    if _1usize == 2 { joystickinfo.b = ButtonStatus::Down; }
                                    if _1usize == 3 { joystickinfo.y = ButtonStatus::Down; }
                                    if _1usize == 5 { joystickinfo.rb = ButtonStatus::Down; }
                                    if _1usize == 7 { joystickinfo.lb = ButtonStatus::Down; }
                                    if _1usize == 4 { joystickinfo.lt = ButtonStatus::Down; }
                                    if _1usize == 6 { joystickinfo.rt = ButtonStatus::Down; }
                                },
                                State::Released => {
                                    if _1usize == 0 { joystickinfo.x = ButtonStatus::Up; }
                                    if _1usize == 1 { joystickinfo.a = ButtonStatus::Up; }
                                    if _1usize == 2 { joystickinfo.b = ButtonStatus::Up; }
                                    if _1usize == 3 { joystickinfo.y = ButtonStatus::Up; }
                                    if _1usize == 5 { joystickinfo.rb = ButtonStatus::Up; }
                                    if _1usize == 7 { joystickinfo.lb = ButtonStatus::Up; }
                                    if _1usize == 4 { joystickinfo.lt = ButtonStatus::Up; }
                                    if _1usize == 6 { joystickinfo.rt = ButtonStatus::Up; }
                                },
                                _=>{}
                            }
                        },
                        multiinput::event::RawEvent::JoystickAxisEvent( _usize_id, _Axis,  _f64)=>{ 
                            use multiinput::event::Axis;
                            match _Axis{ 
                                Axis::X => { joystickinfo.axis_x = _f64 as f32; },
                                Axis::Y => { joystickinfo.axis_y = _f64 as f32; },
                                _=>{}
                            }
                        },
                        multiinput::event::RawEvent::JoystickHatSwitchEvent(_usize_id, _HatSwitch)=>{ println!("hatswitch {:?}", _HatSwitch); },
                        _=>{}
                    }
                }

                renderDefaultToBuffer(&mut GLOBAL_BACKBUFFER, None);

                if app_main(&mut app_data, &keyboardinfo, &joystickinfo, &textinfo, &mouseinfo, nframes, time_instance.elapsed(), window_handle) != 0 { break 'a; }

                //NOTE
                //Whats the difference between get client rect and get window rect
                //and why does client rect give bad x and y values
                let device_context = user32::GetDC(window_handle);
                let mut rect: RECT = RECT{ left: 0, top: 0, right: 0, bottom: 0};
                user32::GetClientRect(window_handle, &mut rect as *mut RECT);
                update_window(device_context, &GLOBAL_BACKBUFFER, 0, 0, rect.right-rect.left, rect.bottom-rect.top);

                if user32::GetWindowRect(window_handle, &mut rect) != 0{
                    GLOBAL_WINDOWINFO.x = rect.left;
                    GLOBAL_WINDOWINFO.y = rect.top;
                    GLOBAL_WINDOWINFO.w = rect.right - rect.left;
                    GLOBAL_WINDOWINFO.h = rect.bottom - rect.top;
                }

                user32::ReleaseDC(window_handle, device_context);
                nframes += 1;
            }
        } else{

        }
    } else{

    }
}}


struct WindowInfo{
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    //name: String
}

fn main() {

    //setprivilege();
    use stb_tt_sys::stbtt_InitFont as InitFont;
    let mut f = File::open("assets/NotoSans-Regular.ttf").expect("assects/NotoSans-Regular.ttf could not be opened.");
    let mut font_buffer = Vec::new();
    f.read_to_end(&mut font_buffer).expect("Font could not be read into buffer.");
    unsafe{
        if InitFont(&mut GLOBAL_FONTINFO as *mut stbtt_fontinfo, font_buffer.as_ptr(), 0) == 0{
            panic!("font was not able to load.");
        }
    }
    unsafe { xinput::XInputEnable(1); }
    //TODO Update and remove explicit path
    tensorflow_init(Some("C:\\Users\\thoth\\Documents\\Rust\\screen_capturer\\tensorflow-sys-tools\\tensorflow_assets\\tensorflow.dll\0")).expect("Tensorflow lib init problem.");


    make_window( );
}

//TODO
//fn setprivilege(){
//
//}

#[derive(PartialEq, Clone, Debug)]
enum ButtonStatus{
    Up,
    Down,
    Default
}
impl Default for ButtonStatus{
    fn default()->ButtonStatus{
        ButtonStatus::Default
    }
}

struct MouseInfo{
    x: i32,
    y: i32,
    lbutton: ButtonStatus,
    old_lbutton: ButtonStatus,
    wheel: isize,
    wheel_delta: i32,
}
impl MouseInfo{
    pub fn new()->MouseInfo{
        MouseInfo{
            x: 0,
            y: 0,
            lbutton: ButtonStatus::Default,
            old_lbutton: ButtonStatus::Default,
            wheel: 0,
            wheel_delta: 0,
        }
    }
}
struct TextInfo{
    character: char,
    timing: i32
}

#[derive(PartialEq)]
enum KeyboardEnum{
    Rightarrow,
    Leftarrow,
    Uparrow,
    Downarrow,
    Enter,
    Default
}
struct KeyboardInfo{
    key: Vec<KeyboardEnum>,
    status: Vec<ButtonStatus>,
}

#[derive(Default, Debug)]
struct JoystickInfo{
    x : ButtonStatus,
    y : ButtonStatus,
    a : ButtonStatus,
    b : ButtonStatus,
    lb : ButtonStatus,
    rb : ButtonStatus,
    lt : ButtonStatus,
    rt : ButtonStatus,

    axis_x: f32,
    axis_y: f32,
}

#[derive(Clone)]
struct TextBox{
    text_buffer: String,
    text_cursor: usize,
    max_char: i32,
    max_render_length: i32,
    text_size: f32,
    x: i32,
    y: i32,
    text_color:[f32;4],
    bg_color:[f32;4],
    cursor_color:[f32;4],
    active: bool,
}
impl TextBox{
    pub fn new()->TextBox{
        TextBox{
            text_buffer: String::new(),
            text_cursor: 0,
            max_char: 30,
            max_render_length: 200,
            text_size: 24.0,
            x: 0,
            y: 0,
            text_color:[0.5;4],
            cursor_color:[0.8;4],
            bg_color:[1.0, 1.0, 1.0, 0.1],
            active: false,
        }
    }
    pub fn update(&mut self, keyboardinfo : &KeyboardInfo, textinfo: &TextInfo, mouseinfo: &MouseInfo){
        fn placeCursor(_self: &mut TextBox, mouseinfo: &MouseInfo){//Look for where to place cursor
            let mut position = 0;
            for (i, it) in _self.text_buffer.chars().enumerate() {
                //IF mouse is between old position and new position then we place cursor
                //behind the current character
                let adv = getAdvance(it, _self.text_size);
                if i < _self.text_buffer.len() - 1{
                    if mouseinfo.x >= position + _self.x+2 && mouseinfo.x < position + adv + _self.x + 2 {
                        _self.text_cursor = i;
                        break;
                    }
                } else{
                    if mouseinfo.x >= position + _self.x+2 {
                        _self.text_cursor = i + 1;
                        break;
                    }
                }

                position += adv;
            }
        }


        if self.active == false {
            if in_rect(mouseinfo.x, mouseinfo.y,
               [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32]) &&
               mouseinfo.lbutton == ButtonStatus::Down{
                self.active = true;

                placeCursor(self, mouseinfo);
            }
            return;
        }


        if  self.active {
            if in_rect(mouseinfo.x, mouseinfo.y,
                [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32]) == false &&
                mouseinfo.lbutton == ButtonStatus::Down{
                self.active = false;
                return;
            } else { //IS THIS A GOOD ELSE STATEMENT I DON'T THINK THIS MAKES SENSE
                if in_rect(mouseinfo.x, mouseinfo.y,
                   [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32]) &&
                   mouseinfo.lbutton == ButtonStatus::Down
                {//Look for where to place cursor
                    placeCursor(self, mouseinfo);

                }

                for i in 0..keyboardinfo.key.len(){
                    if  keyboardinfo.key[i] == KeyboardEnum::Enter &&
                       keyboardinfo.status[i] == ButtonStatus::Down {
                        self.active = false;
                        return;
                    }
                }
            }
        }

        for i in 0..keyboardinfo.key.len(){
            if keyboardinfo.status[i] == ButtonStatus::Down{
                if keyboardinfo.key[i] == KeyboardEnum::Leftarrow{
                    if self.text_cursor > 0 {
                        self.text_cursor -= 1;
                    }
                }
                if keyboardinfo.key[i] == KeyboardEnum::Rightarrow{
                    if (self.text_cursor as usize) < self.text_buffer.len() {
                        self.text_cursor += 1;
                    }
                }
            }
        }

        if textinfo.character != '\0'{
            let _cursor = self.text_cursor as usize;
            if (textinfo.character as u8 == 8)  && (self.text_buffer.len() > 0){
                self.text_buffer.remove(_cursor-1);
                self.text_cursor -= 1;
            } else {
                if self.text_buffer.len() < self.max_char as usize {
                    self.text_buffer.insert(_cursor, textinfo.character);
                    self.text_cursor += 1;
                }
            }
            if self.text_cursor as usize > self.text_buffer.len() {
                self.text_cursor = self.text_buffer.len();
            }
        }

    }
    pub fn draw(&self, time: f32){unsafe{
        drawRect(&mut GLOBAL_BACKBUFFER,
             [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32],
             self.bg_color, true);
        drawString(&mut GLOBAL_BACKBUFFER, &self.text_buffer, self.x, self.y, self.text_color, self.text_size);


        if self.active {
            let mut adv = 0;
            let mut cursor_color = self.cursor_color;
            cursor_color[3] = cursor_color[3] * ( 0.5*(time/3.0e8).cos() + 0.7).min(1.0);

            for (i, it) in self.text_buffer.chars().enumerate(){

                if i == self.text_cursor as usize {
                    drawRect(&mut GLOBAL_BACKBUFFER, [self.x+adv+4, self.y+4, 2, self.text_size as i32],
                         cursor_color, true);
                    break;
                }
                adv += getAdvance(it, self.text_size);
            }
            if self.text_buffer.len() == 0 || self.text_cursor == self.text_buffer.len(){
                drawRect(&mut GLOBAL_BACKBUFFER, [self.x+adv+4, self.y+4, 2, self.text_size as i32],
                     cursor_color, true);
            }
        }
    }}
}

//TODO
//Move some where generally
//I should put together a really simple single file lib
//that hold all the holds all the funcs and structs that
// I like but rust doesn't offer
struct TGMap<T, V>{
    keys: Vec<T>,
    values: Vec<V>
}
impl<T: PartialEq, V> TGMap<T, V>{
    pub fn new()->TGMap<T,V>{
        TGMap{
            keys: Vec::with_capacity(10),
            values: Vec::with_capacity(10),
        }
    }

    pub fn insert_or_set(&mut self, key: T, value: V){
        for (i, it) in self.keys.iter().enumerate(){
            if *it  == key{
                self.values[i] = value;
                return;
            }
        }
        self.keys.push(key);
        self.values.push(value);
        return;
    }
    pub fn get_mut(&mut self, key: T)->&mut V{
        for (i, it) in self.keys.iter().enumerate(){
            if *it  == key{
                return &mut self.values[i];
            }
        }
        println!("Something is not map");
        return &mut self.values[0];
    }
    pub fn len(&self)->usize{
        self.keys.len()
    }
}


#[derive(PartialEq, Debug)]
enum TriggerType{
    Keyboard,
    Gamepad,
}

#[derive(Debug)]
struct CameraTrigger{
    id: u32,
    trigger_type: TriggerType,
    recently_updated: usize,
}

struct AiAppData{
    init: bool,
    glyph_model: TGBasicModel,
    character_portrait_model: TGBasicModel,
    character_portrait_map: TGMap<i32, String>,
    glyph_diagnostics_render: bool,
    meters_diagnostics_render: bool,
    portrait_diagnostics_render: bool,

}
impl  AiAppData{
    fn new()->AiAppData{
        AiAppData{
            init: false,
            glyph_model: TGBasicModel::new(),
            character_portrait_model: TGBasicModel::new(),
            character_portrait_map: TGMap::new(),
            glyph_diagnostics_render: false,
            meters_diagnostics_render: false,
            portrait_diagnostics_render: true,
        }
    }
}

struct RectResultsStorage{
    image_name: String,
    rects : [[i32; 4]; 10],
    app_rects : [[i32; 4]; 10],
}
impl RectResultsStorage{
    fn new()->RectResultsStorage{
        RectResultsStorage{
            image_name: String::new(),
            rects: [[0i32; 4]; 10],
            app_rects: [[0i32; 4]; 10],
        }
    }
}
struct RectangleAppData{
    init: bool,
    active: bool,//TODO rename
    xy_or_wh: bool,
    nth_file: usize,
    nth_player: usize, //TODO rename
    are_we_writing: isize,
    bmp_box: [i32;4],
    temp_bmp_w_h: [i32;2],
    rects: [[i32;4]; 10],
    _rects: [[i32;4]; 10],
    _temp_rect: [i32;4],
    folder_path_textbox: TextBox,
    active_bmp: TGBitmap,
    active_bmp_init: bool,
    active_bmp_name: String,

    storage : Vec<RectResultsStorage>,
}
impl RectangleAppData{
    fn new()->RectangleAppData{
        RectangleAppData{
            init: false,
            active: false,
            xy_or_wh: true,
            nth_file: 0usize,
            nth_player: 0usize,
            are_we_writing: -1isize,
            bmp_box: [250, 50, 650, 400],
            temp_bmp_w_h: [0,0],
            rects: [[0i32;4]; 10],
            _rects: [[0i32;4]; 10],//TODO fix name
            _temp_rect: [0i32;4],
            folder_path_textbox: TextBox::new(),
            active_bmp: TGBitmap::new(0,0),
            active_bmp_init: false,
            active_bmp_name: String::new(),

            storage : vec![],
        }
    }
    fn set_rects(&mut self, image_name: &str){
        let mut in_storage = false;
        for it in self.storage.iter(){
            if it.image_name == image_name {
                 self._rects = it.rects;
                 self.rects = it.app_rects;
                 return;
             } //Change to replace
        }

    }
    fn store(&mut self, image_name: &str){
        let mut in_storage = false;
        for it in self.storage.iter_mut(){
            if it.image_name == image_name {
                 it.rects = self._rects;
                 it.app_rects = self.rects;
                 return;
             } //Change to replace
        }
        self.storage.push( RectResultsStorage{image_name: image_name.to_string(),
                                              rects: self._rects,
                                              app_rects: self.rects} );
    }
    fn write(&self, filename: &str){
        let mut contents = String::new();
        contents += "image hash";
        for i in 0..self.rects.len(){
            contents += &format!{",x{0}, y{0}, w{0}, h{0}", i};
        }
        contents += "\n";
        for it in self.storage.iter(){
            contents += &it.image_name;
            for jt in it.rects.iter(){
                contents += &format!{",{},{},{},{}", jt[0], jt[1], jt[2], jt[3]};
            }
            contents += "\n";
        }
        let mut f = File::create(&format!("{}.txt",filename)).expect("Could not create rect result storage file");
        f.write(contents.as_bytes());
    }
}




const MAX_SOUND_DATA :usize = 48000*2*4;
struct CassetteTape {
    begin_marker: usize, 
    end_marker  : usize, 
    data  : Vec<f32>,
}

impl CassetteTape{
    pub fn new()->CassetteTape{
        CassetteTape {
            begin_marker: std::usize::MAX, 
            end_marker  : std::usize::MAX, 
            data        : vec![0f32; MAX_SOUND_DATA],
        }
    }
    pub fn save(&self, filename: &str){
        let mut filebuffer = File::create(filename).expect("Could not write file.");
        filebuffer.write( &transmute(&['C' as std::os::raw::c_char, 'T' as std::os::raw::c_char]) );
        filebuffer.write( &transmute(&self.begin_marker) );
        filebuffer.write( &transmute(&self.end_marker) );

        let data_len = self.data.len();
        let data = &self.data[0] as *const f32 as *const u8;
        let _data = unsafe{ std::slice::from_raw_parts(data, data_len*4) };
        filebuffer.write( _data );
        
    }
}

struct SoundAppData{
    init : bool,

    enumerator_ptr   : *mut winapi::ctypes::c_void,
    device_ptr       : *mut IMMDevice,
    audioclient_ptr  : *mut winapi::ctypes::c_void,
    audio_prefs      : *mut WAVEFORMATEX,
    captureclient_ptr: *mut winapi::ctypes::c_void,
    recorded_buffer  : Vec<f32>, //; MAX_SOUND_DATA]>,

    record_on: bool,
    to_be_filled: isize,
    button_press_marker: usize,

    tape_deck: TGMap<String, CassetteTape>,//TODO add marker to tape 
    textbox  : TextBox,

    dsound_obj : dsound::LPDIRECTSOUND,

}

impl SoundAppData{
    pub fn new()->SoundAppData{
        SoundAppData{
            init: false,

            enumerator_ptr   : null_mut(),
            device_ptr       : null_mut(),
            audioclient_ptr  : null_mut(),
            audio_prefs      : null_mut(),
            captureclient_ptr: null_mut(),
            recorded_buffer  : vec![0f32; MAX_SOUND_DATA],
  
            record_on           : false,
            to_be_filled        : MAX_SOUND_DATA as isize,
            button_press_marker : std::usize::MAX,

            tape_deck : TGMap::new(), 
            textbox   : TextBox::new(),

            dsound_obj       : null_mut(),
        }
    }
}


#[derive(Debug)]
struct ColorVector{
    r : f32,
    g : f32,
    b : f32,

    avg_intensity : f32,
    count         : i32,
}

impl ColorVector{
    fn init(r: u8, g: u8, b: u8)->ColorVector{
        let _r = r as f32;
        let _g = g as f32;
        let _b = b as f32;
        let intensity = (_r.powf(2.0) + _g.powf(2.0) + _b.powf(2.0)).powf(0.5) + 0.001;
        ColorVector{
            r : _r / intensity, 
            g : _g / intensity, 
            b : _b / intensity, 

            avg_intensity : intensity, 
            count : 1,
        }
    }
    fn rgba(&self)->[f32;4]{
        [self.r, self.g, self.b, 1.0]
    }
}


struct ColorVecAppData{
    init    : bool,

    bmp      : TGBitmap,
    small_bmp: TGBitmap,
    alt_bmp  : TGBitmap,

    fix_color_vectors : bool,
  
    color_vector_set: Vec::<ColorVector>,
    ch1_color_vector_set: Vec::<ColorVector>,
    ch2_color_vector_set: Vec::<ColorVector>,

    filename_textbox: TextBox,
    old_filename: String,

}

impl ColorVecAppData{
    pub fn new()->ColorVecAppData{
        ColorVecAppData{
            init : false,
            bmp  : TGBitmap::new(0,0),
            small_bmp  : TGBitmap::new(0,0),
            alt_bmp  : TGBitmap::new(0,0),
            fix_color_vectors : false,
            color_vector_set :  Vec::with_capacity(100),
            ch1_color_vector_set :  Vec::with_capacity(100),
            ch2_color_vector_set :  Vec::with_capacity(100),

            filename_textbox: TextBox::new(),
            old_filename: String::new(),
        }
    }
}




enum MenuEnum{
    screenshot,
    ai,
    rect,
    sound,
    colorvec,
}

struct MenuData{
    actived: bool,
    apps: Vec<String>,
}
impl MenuData{
    fn new()->MenuData{
        MenuData{
            actived: false,
            apps: vec!["screenshot".to_string(), "ai".to_string(), "rect".to_string()],
        }
    }
}

struct AppData{
    //TODO
    //Move screensot, ai, and other app setting and data are sectioned off
    current_app: MenuEnum,
    global_menu_data: MenuData,

    //SCREENSHOTE DATA//
    capture_exe_textbox: TextBox,
    capture_exe_update_text: String,
    capture_exe_update_text_color: [f32; 4],
    capture_exe_update_text_size: f32,
    capture_exe_update_box: [i32;4],
    capture_exe_update_box_color: [f32; 4],

    root_folder_textbox: TextBox,
    root_folder_update_text: String,
    root_folder_update_text_color: [f32;4],
    root_folder_update_text_size: f32,
    root_folder_update_box: [i32;4],
    root_folder_update_box_color: [f32;4],

    image_prepend_name_textbox: TextBox,
    image_counter : usize,

    number_of_shots_to_take_textbox: TextBox,

    screenshot_buffer: Vec<TGBitmap>,
    currently_rendering_screenshot: Option<TGBitmap>,
    old_rendering_index: usize,
    currently_rendering_index: usize,

    arrow_right_bmp: TGBitmap,
    arrow_left_bmp: TGBitmap,
    arrow_right_alpha: f32,
    arrow_left_alpha: f32,

    gamepad_bmp: TGBitmap,
    keyboard_bmp: TGBitmap,
    cameratrigger: CameraTrigger,
    cameratrigger_is_updating: bool,

    temp_bmp: TGBitmap,
    //SCREENSHOT DATA//

    //AI DATA//
    ai_data: AiAppData,
    rect_data: RectangleAppData,
    sound_data: SoundAppData,
    colorvec_data: ColorVecAppData,

    handle_dc: Option<WindowHandleDC>,
}
impl AppData{
    pub fn new()->AppData{
        let mut capture_exe_textbox = TextBox::new();
        capture_exe_textbox.text_buffer = "Guilty Gear Xrd -REVELATOR-".to_string();
        //capture_exe_textbox.text_buffer = "MELTY BLOOD Actress Again Current Code".to_string();
        //capture_exe_textbox.text_buffer = "Skullgirls Encore".to_string();
        capture_exe_textbox.text_buffer = "SSFIVAE".to_string();

        let mut root_folder_textbox = TextBox::new();
        root_folder_textbox.text_buffer = "temp".to_string();

        let mut image_prepend_name_textbox = TextBox::new();
        image_prepend_name_textbox.text_buffer = "text".to_string();

        let mut number_of_shots_to_take_textbox = TextBox::new();
        number_of_shots_to_take_textbox.text_buffer = "1".to_string();


        AppData{
            current_app: MenuEnum::colorvec,
            global_menu_data: MenuData::new(),

            capture_exe_textbox: capture_exe_textbox,
            capture_exe_update_text: String::new(),
            capture_exe_update_text_color: [1.0, 1.0, 1.0, 0.0],
            capture_exe_update_text_size: 20.0,
            capture_exe_update_box: [0; 4],
            capture_exe_update_box_color: [1.0, 0.2, 0.0, 0.0],

            root_folder_textbox: root_folder_textbox,
            root_folder_update_text: String::new(),
            root_folder_update_text_color: [1.0, 1.0, 1.0, 0.0],
            root_folder_update_text_size: 20.0,
            root_folder_update_box: [0, 0, 0, 0],
            root_folder_update_box_color: [1.0, 0.2, 0.0, 0.0],

            image_prepend_name_textbox: image_prepend_name_textbox,
            image_counter : 0,

            number_of_shots_to_take_textbox: number_of_shots_to_take_textbox,

            screenshot_buffer: Vec::new(),
            currently_rendering_screenshot: None,
            old_rendering_index: 0,
            currently_rendering_index: 0,

            arrow_right_bmp: loadBMP("assets/arrow_right.bmp"),
            arrow_left_bmp: loadBMP("assets/arrow_left.bmp"),
            arrow_right_alpha: 0.0,
            arrow_left_alpha: 0.0,

            gamepad_bmp : loadBMP("assets/gamepad.bmp"),
            keyboard_bmp: loadBMP("assets/keyboard.bmp"),
            cameratrigger: CameraTrigger{id: 0x20,
                                        trigger_type: TriggerType::Keyboard,
                                        recently_updated: 0},
            cameratrigger_is_updating: false,

            temp_bmp: loadBMP("assets/Untitled.bmp"),

            ai_data: AiAppData::new(),
            rect_data: RectangleAppData::new(),

            sound_data: SoundAppData::new(),

            colorvec_data: ColorVecAppData::new(),

            handle_dc: None,
        }

    }
}
fn menu(app_data: &mut AppData, keyboardinfo: &KeyboardInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration){unsafe{
    let x = GLOBAL_BACKBUFFER.w-25;
    let y = GLOBAL_BACKBUFFER.h-25;
    //The following 5 draw rect calls draws a shitty menu icon
    drawRect(&mut GLOBAL_BACKBUFFER, [x, y, 20, 20],
                                    [0.7, 0.7, 0.7, 1.0], true);
    drawRect(&mut GLOBAL_BACKBUFFER, [x+2, y+15, 5, 5],
                                    [0.9, 0.9, 0.9, 1.0], true);
    drawRect(&mut GLOBAL_BACKBUFFER, [x+2, y+7, 5, 5],
                                    [0.9, 0.9, 0.9, 1.0], true);
    drawRect(&mut GLOBAL_BACKBUFFER, [x+9, y+15, 10, 5],
                                    [0.9, 0.9, 0.9, 1.0], true);
    drawRect(&mut GLOBAL_BACKBUFFER, [x+9, y+7, 10, 5],
                                    [0.9, 0.9, 0.9, 1.0], true);
    if in_rect(mouseinfo.x, mouseinfo.y, [x, y, 20, 20]) && mouseinfo.lbutton == ButtonStatus::Down{
        app_data.global_menu_data.actived = true;
    }
    if app_data.global_menu_data.actived == true{
        let x = 100;
        let y = 50;
        if in_rect(mouseinfo.x, mouseinfo.y, [x, y, 500, 500]) == false &&
         mouseinfo.lbutton == ButtonStatus::Down && mouseinfo.old_lbutton != mouseinfo.lbutton{
            app_data.global_menu_data.actived = false;
        }
        drawRect(&mut GLOBAL_BACKBUFFER, [x, y, 500, 500],
                                    [0.5, 0.5, 0.5, 0.7], true);
        drawString(&mut GLOBAL_BACKBUFFER, "Menu", x + 50, y + 400,
         [0.9, 0.9, 0.0, 1.0], 42.0);
        let screenshot_len = drawString(&mut GLOBAL_BACKBUFFER, "Screenshot APP", x + 50, y + 300,
         [0.9, 0.9, 0.0, 1.0], 32.0);
         if in_rect(mouseinfo.x, mouseinfo.y, [x + 50, y + 300, screenshot_len, 32]){
            drawRect(&mut GLOBAL_BACKBUFFER, [x + 48, y+300, screenshot_len+4, 2],
                                            [0.9, 0.9, 0.9, 1.0], true);
            if mouseinfo.lbutton == ButtonStatus::Down{
                app_data.current_app = MenuEnum::screenshot;
            }
         }
        let ai_len = drawString(&mut GLOBAL_BACKBUFFER, "Ai APP", x + 50, y + 200,
         [0.9, 0.9, 0.0, 1.0], 32.0);
         if in_rect(mouseinfo.x, mouseinfo.y, [x + 50, y + 200, ai_len, 32]){
            drawRect(&mut GLOBAL_BACKBUFFER, [x + 48, y+200, ai_len+4, 2],
                                            [0.9, 0.9, 0.9, 1.0], true);
            if mouseinfo.lbutton == ButtonStatus::Down{
                app_data.current_app = MenuEnum::ai;
            }
         }
        let rect_len = drawString(&mut GLOBAL_BACKBUFFER, "Rect APP", x + 50, y + 100,
         [0.9, 0.9, 0.0, 1.0], 32.0);
         if in_rect(mouseinfo.x, mouseinfo.y, [x + 50, y + 100, rect_len, 32]){
            drawRect(&mut GLOBAL_BACKBUFFER, [x + 48, y+100, rect_len+4, 2],
                                            [0.9, 0.9, 0.9, 1.0], true);
            if mouseinfo.lbutton == ButtonStatus::Down{
                app_data.current_app = MenuEnum::rect;
            }
         }
    }
}}
fn app_main(app_data: &mut AppData, keyboardinfo: &KeyboardInfo, joystickinfo: &JoystickInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration, window_handle: HWND)->i32{
    match app_data.current_app{
        MenuEnum::screenshot => {
            return app_screencapture(app_data, keyboardinfo, textinfo, mouseinfo, frames, time_instance);
        },
        MenuEnum::ai => {
            return app_ai(app_data, keyboardinfo, textinfo, mouseinfo, frames, time_instance);
        },
        MenuEnum::rect => {
            return app_rectangle(app_data, keyboardinfo, textinfo, mouseinfo, frames, time_instance);
        },
        MenuEnum::sound => {
            return app_sound(app_data, keyboardinfo, joystickinfo, textinfo, mouseinfo, frames, time_instance, window_handle);
        }
        MenuEnum::colorvec => {
            return app_colorvec(app_data, keyboardinfo, joystickinfo, textinfo, mouseinfo, frames, time_instance, window_handle);
        }
    }
    return 0;
}

#[derive(Clone, Debug)]
struct DbscanPt {
    index: usize,
    class: i8,
}
fn dbscan(set_vec: &mut Vec<DbscanPt>, eps: usize, min_pts: usize)->i8{
    fn rangeQuery(set_vec: &mut Vec<DbscanPt>, pt: usize, eps: usize)->Vec<usize>{
        let mut n = Vec::new();
        for (i, it) in set_vec.iter().enumerate(){
            if it.index == pt { continue; }
            if (it.index as isize - pt as isize).abs() <= eps as isize{
                n.push((i).clone());
            }
        }
        return n;
    }
    //0 == Noise
    //-1== None

    let mut class : i8 = 0;
    for i in 0..set_vec.len(){
        if set_vec[i].class != -1{
            continue;
        } 
        let index = set_vec[i].index;
        let mut neighbor_set = rangeQuery(set_vec, index, eps);
        if neighbor_set.len() < min_pts{
            set_vec[i].class = 0;
            continue;
        }
        
        class += 1;
        set_vec[i].class = class;
        let mut j = 0;
        while j < neighbor_set.len(){
            if set_vec[neighbor_set[j]].class == 0 {set_vec[neighbor_set[j]].class = class;}
            if set_vec[neighbor_set[j]].class != -1 { j+=1; continue;}
            set_vec[neighbor_set[j]].class = class;

            let _index = set_vec[neighbor_set[j]].index;
            let mut _n_set = rangeQuery(set_vec, _index, eps);
            if _n_set.len() >= min_pts{

                for _i in 0.._n_set.len(){
                    let mut in_set = false;
                    for _j in 0..neighbor_set.len(){
                        if _n_set[_i] == neighbor_set[_j]{
                            in_set = true;
                            break;
                        }
                    }
                    if !in_set{
                        neighbor_set.push(_n_set[_i].clone());
                    }
                }
            }
            j +=1;
        }
    }
    return class;
}
fn get_min_index( set: &Vec<DbscanPt>, class: i8)->usize{ //NOTE lets assume things are ordered
    //TODO
    //maybe return a option
    for it in set.iter(){
        if it.class == class{
            return it.index; 
        }
    }
    return std::usize::MAX;
}
fn get_max_index( set: &Vec<DbscanPt>, class: i8)->usize{ //NOTE lets assume things are ordered
    //TODO
    //maybe return a option
    let mut index = 0;
    for it in set.iter(){
        if it.class == class{
            if index < it.index{
               index = it.index; 
            }
        }
    }
    return index;
}

fn app_colorvec(app_data: &mut AppData, keyboardinfo: &KeyboardInfo, joystickinfo: &JoystickInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration, window_handle: HWND)->i32{unsafe{
    let mut colorvec_data = &mut app_data.colorvec_data;
    
    if !colorvec_data.init {
        
        colorvec_data.filename_textbox.text_buffer = "training_testing_data/char_pos_melty/melty_blood_mid_k_1.bmp".to_string();
        colorvec_data.filename_textbox.x = 400;
        colorvec_data.filename_textbox.y = 10;
        colorvec_data.filename_textbox.text_size = 24.0;
        colorvec_data.filename_textbox.max_char = 80;
        colorvec_data.filename_textbox.max_render_length = 500;
        
        colorvec_data.old_filename = colorvec_data.filename_textbox.text_buffer.clone();

        //Load bmp
        colorvec_data.bmp = loadBMP(&colorvec_data.filename_textbox.text_buffer);
        colorvec_data.small_bmp = resizeBMP(&colorvec_data.bmp, 400, 250 );
        colorvec_data.fix_color_vectors = false;

        colorvec_data.init = true; 
    }

    
    colorvec_data.filename_textbox.update(keyboardinfo, textinfo, mouseinfo);
    colorvec_data.filename_textbox.draw(time_instance.subsec_nanos() as f32);
    if colorvec_data.filename_textbox.text_buffer != colorvec_data.old_filename{

        let _path = colorvec_data.filename_textbox.text_buffer.clone();
        let mut _path = std::path::Path::new(&_path);
        if _path.exists() && 
           _path.is_file(){
            colorvec_data.old_filename = colorvec_data.filename_textbox.text_buffer.clone();
            colorvec_data.bmp = loadBMP(&colorvec_data.filename_textbox.text_buffer);
            colorvec_data.small_bmp = resizeBMP(&colorvec_data.bmp, 400, 250 );
        }
    }


    
    //determine list of color vectors
    //NOTE
    //this struct is used to make iteration easier.
    struct RGBA {
        b: u8,
        g: u8,
        r: u8,
        a: u8 
    }
    let size  = (colorvec_data.bmp.width * colorvec_data.bmp.height) as usize;
    let ptr   = colorvec_data.bmp.rgba.as_ptr() as *const RGBA; 
    let slice = std::slice::from_raw_parts(ptr, size);



    let mut color_vector_set = &mut colorvec_data.color_vector_set; 
    let mut ch1_color_vector_set = &mut colorvec_data.ch1_color_vector_set; 
    let mut ch2_color_vector_set = &mut colorvec_data.ch2_color_vector_set; 
    let cut_dr = 0.1;


    let timer = std::time::Instant::now(); 

    let height = colorvec_data.bmp.height;
    let width = colorvec_data.bmp.width;
    //for i in 0..height as usize {
    //    for j in 0..width as usize {
    //        let c = &slice[ j*height as usize + i];
    //        let c_v = ColorVector::init(c.r, c.g, c.b);

    //        let mut in_set = false; 
    //        let mut in_index = 0;
    //        for (_i,it) in color_vector_set.iter().enumerate(){
    //            let dr = ((c_v.r - it.r).powf(2.0) + 
    //                      (c_v.g - it.g).powf(2.0) + 
    //                      (c_v.b - it.b).powf(2.0)).powf(0.5);
  
    //            if dr < cut_dr { 
    //                in_set   = true; 
    //                in_index = _i; 
    //                break; 
    //            }
    //        }
    //        if !in_set { color_vector_set.push(c_v); } 
    //        else {  
    //            let intensity = color_vector_set[in_index].avg_intensity * color_vector_set[in_index].count as f32;
    //            color_vector_set[in_index].count += 1;  
    //            color_vector_set[in_index].avg_intensity = ( intensity + c_v.avg_intensity ) / color_vector_set[in_index].count as f32;
    //        }
    //    }
    //}
    let elapsed = timer.elapsed(); 

    let timer  = std::time::Instant::now(); 
    {//Sub frame analysis
        fn update_color_vector_set( rect: [usize;4], slice: &[RGBA], width: i32, height: i32, color_vector_set: &mut Vec<ColorVector>, cut_dr: f32){
            let x1  = rect[0];
            let y1  = rect[1];
            let x2  = rect[0] + rect[2];
            let y2  = rect[1] + rect[3];
            for i in x1..x2 as usize {
              for j in y1..y2 as usize {
                    let c = &slice[ j*width as usize + i];
                    let c_v = ColorVector::init(c.r, c.g, c.b);

                    let mut in_set = false; 
                    let mut in_index = 0;
                    for (_i,it) in color_vector_set.iter().enumerate(){
                        let dr = ((c_v.r - it.r).powf(2.0) + 
                                  (c_v.g - it.g).powf(2.0) + 
                                  (c_v.b - it.b).powf(2.0)).powf(0.5);
          
                        if dr < cut_dr { 
                            in_set   = true; 
                            in_index = _i; 
                            break; 
                        }
                    }
                    if !in_set { color_vector_set.push(c_v); } 
                    else {  
                        let intensity = color_vector_set[in_index].avg_intensity * color_vector_set[in_index].count as f32;
                        color_vector_set[in_index].count += 1;  
                        color_vector_set[in_index].avg_intensity = ( intensity + c_v.avg_intensity ) / color_vector_set[in_index].count as f32;
                    }
                }
            }
        }

        if colorvec_data.fix_color_vectors == false {
            //TODO 
            //temp numbers; these hard coded numbers are all bad
            {//Estimated background region 
             //NOTE: I have not checked the accuracy of these regions 
                update_color_vector_set( [200, 100, 3*80, 6*80], slice, width, height, &mut color_vector_set, cut_dr);
                update_color_vector_set( [900, 100, 3*80, 6*80], slice, width, height, &mut color_vector_set, cut_dr);
                update_color_vector_set( [640, 100, 1*80, 6*80], slice, width, height, &mut color_vector_set, cut_dr);
            }

            {//Estimated character regions
             //NOTE: I have not checked the accuracy of these regions 
                update_color_vector_set( [6*73,  80, 2*85, 3*100], slice, width, height, &mut ch1_color_vector_set, cut_dr);
                update_color_vector_set( [10*76, 80, 2*85, 3*100], slice, width, height, &mut ch2_color_vector_set, cut_dr);
            }
        }

    }
    let elapsed2 = timer.elapsed(); 



    fn deltaR( c1: &ColorVector, c2: &ColorVector)->f32{
        let dr = ((c1.r - c2.r).powf(2.0) + 
                  (c1.g - c2.g).powf(2.0) + 
                  (c1.b - c2.b).powf(2.0)).powf(0.5);
        
        return dr;
    }

    {//Remove background vectors from signal sample


        let mut n_removed = 0;
        let mut retain_indices = Vec::with_capacity(ch1_color_vector_set.len());

        //NOTE This is not that good....
        for jt in ch1_color_vector_set.iter(){
            let mut color_good = true;
            for it in color_vector_set.iter(){
                //TODO
                //The cull needs tobe done on percentages because image size can change
                if deltaR(it, jt) < 0.05 && it.count > 200 { color_good = false; } 
            }
            retain_indices.push(color_good);  
            if !color_good { n_removed += 1; }
        }
        let mut i=0 ;
        ch1_color_vector_set.retain(|_| (retain_indices[i], i+=1).0);



        let mut retain_indices = Vec::with_capacity(ch1_color_vector_set.len());
        //NOTE This is not that good....
        for jt in ch2_color_vector_set.iter(){
            let mut color_good = true;
            for it in color_vector_set.iter(){
                //TODO
                //The cull needs tobe done on percentages because image size can change
                if deltaR(it, jt) < 0.05 && it.count > 200 { color_good = false; } 
            }
            retain_indices.push(color_good);  
            if !color_good { n_removed += 1; }
        }
        let mut i=0 ;
        ch2_color_vector_set.retain(|_| (retain_indices[i], i+=1).0);
    }



     
    drawString( &mut GLOBAL_BACKBUFFER, &format!("total bkg vectors: {}  ch1 vectors: {}   ch2 vectors: {}", 
                                                  color_vector_set.len(), ch1_color_vector_set.len(), ch2_color_vector_set.len()), 
                500, 475, C4_WHITE, 18.0);
    drawString( &mut GLOBAL_BACKBUFFER, &format!("sub Frame Timer: {:?}   w: {}  h: {}", 
                                                  elapsed2, 5*80, 80), 
                500, 475-18, C4_WHITE, 18.0);
    color_vector_set.sort_by( |b, a| a.count.partial_cmp( &b.count ).unwrap() );

    for (i, it) in color_vector_set.iter().enumerate() { 
        if i > 20 { break; }
        if i == 0 {drawString( &mut GLOBAL_BACKBUFFER, &format!("Background vectors:"), 30, 480 + 10, C4_WHITE, 14.0); }

        drawRect(&mut GLOBAL_BACKBUFFER, [10, 475-10*(i-1) as i32, 20, 10], it.rgba(), true);
        drawString( &mut GLOBAL_BACKBUFFER, &format!("count: {}    avg_intensity  {}", it.count, it.avg_intensity), 30, 480 - 10*i as i32, C4_WHITE, 14.0);
    }
    for (i, it) in ch1_color_vector_set.iter().enumerate() { 
        if i > 20 { break; }
        if i == 0 {drawString( &mut GLOBAL_BACKBUFFER, &format!("character 1 vectors:"), 30, 450 - 9*22, C4_WHITE, 14.0); }
        drawRect(&mut GLOBAL_BACKBUFFER, [10, 450-10*(i+21) as i32, 20, 10], it.rgba(), true);
        drawString( &mut GLOBAL_BACKBUFFER, &format!("count: {}", it.count), 30, 450 - 10*(i+22) as i32, C4_WHITE, 14.0);
    }
    for (i, it) in ch2_color_vector_set.iter().enumerate() { 
        if i > 20 { break; }
        if i == 0 {drawString( &mut GLOBAL_BACKBUFFER, &format!("character 2 vectors:"), 230, 450 - 9*22, C4_WHITE, 14.0); }
        drawRect(&mut GLOBAL_BACKBUFFER, [200, 450-10*(i+21) as i32, 20, 10], it.rgba(), true);
        drawString( &mut GLOBAL_BACKBUFFER, &format!("count: {}   avg_intensity  {}", it.count, it.avg_intensity), 230, 448 - 10*(i+22) as i32, C4_WHITE, 14.0);
    }

    


    {//
        let w_factor = colorvec_data.small_bmp.width as f32 / colorvec_data.bmp.width as f32;
        let h_factor = colorvec_data.small_bmp.height as f32 / colorvec_data.bmp.height as f32;



        let mut vertical_histogram_rm_bkg = vec![0; colorvec_data.small_bmp.width as usize];
        let mut horizontal_histogram_rm_bkg = vec![0; colorvec_data.small_bmp.height as usize];

        let mut vertical_histogram_rm_bkg_ch1 = vec![0; colorvec_data.small_bmp.width as usize];
        let mut horizontal_histogram_rm_bkg_ch1 = vec![0; colorvec_data.small_bmp.height as usize];

        let mut vertical_histogram_rm_bkg_ch2 = vec![0; colorvec_data.small_bmp.width as usize];
        let mut horizontal_histogram_rm_bkg_ch2 = vec![0; colorvec_data.small_bmp.height as usize];
        //Both background and foreground
        if true { for i in 0..colorvec_data.small_bmp.width as usize {
            for j in 0..colorvec_data.small_bmp.height as usize {
                let index = 4*( i + j*colorvec_data.small_bmp.width as usize);
                let r = colorvec_data.small_bmp.rgba[ index + 2];
                let g = colorvec_data.small_bmp.rgba[ index + 1];
                let b = colorvec_data.small_bmp.rgba[ index + 0];
                let cv = ColorVector::init(r, g, b);

                let mut is_ch1 = false;
                let mut is_ch2 = false;

                let mut is_good = false;

                let mut max_count = 0;
                for jt in ch1_color_vector_set.iter(){
                    if deltaR(jt, &cv) < 0.1 {
                        is_good = true;
                        is_ch1 = true;
                        if max_count < jt.count {
                            max_count = jt.count;
                        }
                    }
                }
                for jt in ch2_color_vector_set.iter(){
                    if deltaR(jt, &cv) < 0.1 {
                        is_good = true;
                        is_ch2 = true;
                        if max_count < jt.count {
                            max_count = jt.count;
                        }
                    }
                }

                let mut was_background = false;
                for it in color_vector_set.iter(){
                    let mut temp_is_good = is_good;
                    if it.count > max_count{ temp_is_good = false; }
                    if deltaR(it, &cv) < 0.1 && !temp_is_good{
                        colorvec_data.small_bmp.rgba[ index + 0] = 0;
                        colorvec_data.small_bmp.rgba[ index + 1] = 0;
                        colorvec_data.small_bmp.rgba[ index + 2] = 0;
                        was_background = true;
                        break;
                    }
                }
                if !was_background {
                    vertical_histogram_rm_bkg[i] += 1;
                    if i > 100 && i < colorvec_data.small_bmp.width as usize - 100 { horizontal_histogram_rm_bkg[j] += 1; }

                    if is_ch1 {
                        vertical_histogram_rm_bkg_ch1[i] += 1;
                        if i > 100 && i < colorvec_data.small_bmp.width as usize - 100 { horizontal_histogram_rm_bkg_ch1[j] += 1; }
                    }

                    if is_ch2{
                        vertical_histogram_rm_bkg_ch2[i] += 1;
                        if i > 100 && i < colorvec_data.small_bmp.width as usize - 100 { horizontal_histogram_rm_bkg_ch2[j] += 1; }
                    }
                }
            }
        }}

        //Vertical and Horizontal histograms
        drawRect(&mut GLOBAL_BACKBUFFER, [500, 140, colorvec_data.small_bmp.width, 50], C4_BLACK, true);
        {//Vertical his draw
            let mut mean = 0.0;
            for it in vertical_histogram_rm_bkg.iter(){ 
                mean += (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0);
            }
            mean /= vertical_histogram_rm_bkg.len() as f32;

            let mut std = 0.0;
            for it in vertical_histogram_rm_bkg.iter(){ 
                std += ((*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) - mean).powf(2.0);
             }
            std /= vertical_histogram_rm_bkg.len() as f32;


            let mut dbscan_vec = Vec::new();

            for (i, it) in vertical_histogram_rm_bkg.iter().enumerate(){
                let _y_offset = (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0);
                let y_offset = (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) as i32;
                if _y_offset >  mean {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], C4_GREEN, true);
                    dbscan_vec.push(DbscanPt{index: i, class: -1});
                }
                else if _y_offset >  mean + std {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], C4_RED, true);
                }
                else {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], C4_WHITE, true);
                }
            }

            let c = dbscan(&mut dbscan_vec, 8, 7);

            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_min_index(&dbscan_vec, 3) as i32, 140, 2, 50], C4_YELLOW, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_max_index(&dbscan_vec, 3) as i32, 140, 2, 50], C4_YELLOW, true);

            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_min_index(&dbscan_vec, 2) as i32, 140, 2, 50], C4_YELLOW, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_max_index(&dbscan_vec, 2) as i32, 140, 2, 50], C4_YELLOW, true);
        } 
        {//Vertical his draw
            let mut mean = 0.0;
            for it in vertical_histogram_rm_bkg_ch2.iter(){ 
                mean += (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0);
            }
            mean /= vertical_histogram_rm_bkg_ch1.len() as f32;

            let mut std = 0.0;
            for it in vertical_histogram_rm_bkg_ch2.iter(){ 
                std += ((*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) - mean).powf(2.0);
             }
            std /= vertical_histogram_rm_bkg_ch2.len() as f32;


            let mut dbscan_vec = Vec::new();

            for (i, it) in vertical_histogram_rm_bkg_ch2.iter().enumerate(){
                let _y_offset = (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0);
                let y_offset = (*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) as i32;
                if _y_offset >  mean {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], [0.8, 0.1, 0.8, 0.5], true);
                    dbscan_vec.push(DbscanPt{index: i, class: -1});
                }
                else if _y_offset >  mean + std {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], C4_RED, true);
                }
                else {
                    drawRect(&mut GLOBAL_BACKBUFFER, [500 + i as i32, 140+y_offset, 2, 2], C4_GREY, true);
                }
            }

            let c = dbscan(&mut dbscan_vec, 8, 7);

            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_min_index(&dbscan_vec, 3) as i32, 140, 2, 50], C4_GREEN, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_max_index(&dbscan_vec, 3) as i32, 140, 2, 50], C4_GREEN, true);

            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_min_index(&dbscan_vec, 2) as i32, 140, 2, 50], C4_GREEN, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [500 + get_max_index(&dbscan_vec, 2) as i32, 140, 2, 50], C4_GREEN, true);
        } 
        drawRect(&mut GLOBAL_BACKBUFFER, [440, 200, 50, colorvec_data.small_bmp.height], C4_BLACK, true);
        {//Horizontal his draw
            let mut mean = 0.0;
            for it in horizontal_histogram_rm_bkg.iter(){ 
                mean += (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0);
            }
            mean /= horizontal_histogram_rm_bkg.len() as f32;

            let mut std = 0.0;
            for it in horizontal_histogram_rm_bkg.iter(){ 
                std += ((*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) - mean).powf(2.0);
             }
            std /= horizontal_histogram_rm_bkg.len() as f32;

            let mut dbscan_vec = Vec::new();
            for (i, it) in horizontal_histogram_rm_bkg.iter().enumerate(){
                let _x_offset = (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0);
                let x_offset = (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0) as i32;

                if _x_offset >  mean {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], C4_GREEN, true);
                    dbscan_vec.push(DbscanPt{index: i, class: -1});
                }
                else if _x_offset >  mean + std {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], C4_RED, true);
                }
                else {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], C4_WHITE, true);
                }
            }

            let c = dbscan(&mut dbscan_vec, 8, 7);

            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_min_index(&dbscan_vec, 1) as i32, 50, 2], C4_YELLOW, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_max_index(&dbscan_vec, 1) as i32, 50, 2], C4_YELLOW, true);
                                                                                             
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_min_index(&dbscan_vec, 2) as i32, 50, 2], C4_YELLOW, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_max_index(&dbscan_vec, 2) as i32, 50, 2], C4_YELLOW, true);
        } 
        {//Horizontal hist draw remove char 2?
            let mut mean = 0.0;
            for it in horizontal_histogram_rm_bkg_ch2.iter(){ 
                mean += (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0);
            }
            mean /= horizontal_histogram_rm_bkg_ch2.len() as f32;

            let mut std = 0.0;
            for it in horizontal_histogram_rm_bkg_ch2.iter(){ 
                std += ((*it as f32 / colorvec_data.small_bmp.height as f32 * 50.0) - mean).powf(2.0);
             }
            std /= horizontal_histogram_rm_bkg_ch2.len() as f32;

            let mut dbscan_vec = Vec::new();
            for (i, it) in horizontal_histogram_rm_bkg_ch2.iter().enumerate(){
                let _x_offset = (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0);
                let x_offset = (*it as f32 / colorvec_data.small_bmp.width as f32 * 50.0) as i32;

                if _x_offset >  mean {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], [0.8, 0.0, 0.8, 0.5], true);
                    dbscan_vec.push(DbscanPt{index: i, class: -1});
                }
                else if _x_offset >  mean + std {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], C4_RED, true);
                }
                else {
                    drawRect(&mut GLOBAL_BACKBUFFER, [440 + x_offset as i32, 200+i as i32, 2, 2], C4_GREY, true);
                }
            }

            let c = dbscan(&mut dbscan_vec, 8, 7);

            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_min_index(&dbscan_vec, 1) as i32, 50, 2], C4_GREEN, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_max_index(&dbscan_vec, 1) as i32, 50, 2], C4_GREEN, true);
                                                                                             
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_min_index(&dbscan_vec, 2) as i32, 50, 2], C4_GREEN, true);
            drawRect(&mut GLOBAL_BACKBUFFER, [440, 200 + get_max_index(&dbscan_vec, 2) as i32, 50, 2], C4_GREEN, true);
        }
    }



    {//Toggle fixing color vector models.
        if in_rect(mouseinfo.x, mouseinfo.y, [500, 100, 300, 30]){
            drawRect(&mut GLOBAL_BACKBUFFER, [500, 100, 300, 30], C4_WHITE, false);
            if mouseinfo.lbutton == ButtonStatus::Down && mouseinfo.old_lbutton == ButtonStatus::Up{
                colorvec_data.fix_color_vectors = !colorvec_data.fix_color_vectors;
            }
        }

        let offset_str  = drawString(&mut GLOBAL_BACKBUFFER, "Color Vector model is : ", 500, 100, C4_WHITE, 24.0 );
        if colorvec_data.fix_color_vectors{
            drawString(&mut GLOBAL_BACKBUFFER, "FIXED", 500+offset_str, 100, C4_WHITE, 24.0);

        } else{
            drawString(&mut GLOBAL_BACKBUFFER, "NOT-FIXED", 500+offset_str, 100, C4_WHITE, 24.0);

            color_vector_set.clear();
            ch1_color_vector_set.clear();
            ch2_color_vector_set.clear();
        }
    }



    drawBMP( &mut GLOBAL_BACKBUFFER, &mut colorvec_data.small_bmp, 500, 200, 1.0, None, None);
    return 0;
}}



fn app_sound(app_data: &mut AppData, keyboardinfo: &KeyboardInfo, joystickinfo: &JoystickInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration, window_handle: HWND)->i32{unsafe{
    //TODO
    //save output

    let REFTIMES_PER_SEC  = 10000000;
    let REFTIMES_PER_MILLISEC  = 10000;


    let requested_duration = REFTIMES_PER_SEC;

    drawRect(&mut GLOBAL_BACKBUFFER, [0, 0, GLOBAL_BACKBUFFER.w, GLOBAL_BACKBUFFER.h], [0.05, 0.2, 0.05, 1.0], true);
    drawString(&mut GLOBAL_BACKBUFFER, "SOUNDS like a sailor", 300, 450, [1.0, 1.0, 1.0, 1.0], 34.0);



    //The following code is roughly based off of example code found in 
    //https://docs.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream
    //use winapi::um::combaseapi::{CoCreateInstance, CLSCTX_ALL};
    //use winapi::um::mmdeviceapi::{CLSID_MMDeviceEnumerator, IMMDeviceEnumerator, eRender, eConsole, IMMDevice};
    //use winapi::um::audioclient::{IID_IAudioClient, IAudioClient, IID_IAudioCaptureClient, IAudioCaptureClient, AUDCLNT_BUFFERFLAGS_SILENT};
    //use winapi::um::audiosessiontypes::{AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_SHAREMODE_SHARED};
    //use winapi::shared::guiddef::GUID;
    //use winapi::shared::mmreg::WAVEFORMATEX;
    //use winapi::Interface;

    macro_rules! deref{
        ($x:expr, $y:ty)=>{
            (*($x as *mut $y))
        }
    }


    let sound_data = &mut app_data.sound_data;
    if sound_data.init == false {
        sound_data.init = true;

        sound_data.textbox.text_buffer += "demo";
        sound_data.textbox.x = 50;
        sound_data.textbox.y = 370;



        let hr = CoCreateInstance(&CLSID_MMDeviceEnumerator as *const _, null_mut(), CLSCTX_ALL, &IMMDeviceEnumerator::uuidof() as *const _,  
                                  &mut sound_data.enumerator_ptr as *mut _);
        assert!(hr==0,"Result of CoCreateInstance {} {:?} {}", hr, sound_data.enumerator_ptr, sound_data.init);


        let hr = deref!(sound_data.enumerator_ptr, IMMDeviceEnumerator).GetDefaultAudioEndpoint(eRender, eConsole, &mut sound_data.device_ptr as *mut _);
        assert!(hr==0,"Result of GetDefaultAudioEndpoint {}", hr);
        
        let hr = deref!(sound_data.device_ptr, IMMDevice).Activate(&IID_IAudioClient as *const _, CLSCTX_ALL, null_mut(), &mut sound_data.audioclient_ptr as *mut _);
        assert!(hr==0,"Result of Active {}", hr);
        
        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).GetMixFormat(&mut sound_data.audio_prefs as *mut _);
        assert!(hr==0,"Result of GetMixFormat {}", hr);


        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).Initialize(
                             AUDCLNT_SHAREMODE_SHARED,
                             AUDCLNT_STREAMFLAGS_LOOPBACK,
                             requested_duration,
                             0,
                             sound_data.audio_prefs as *mut _,
                             null_mut()); 
        assert!(hr==0,"Result of Initiaize {}", hr);


        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).GetService(
                         &IID_IAudioCaptureClient as *const _,
                         &mut sound_data.captureclient_ptr as *mut _);
        assert!(hr==0,"Result of GetService {}", hr);


        //TODO
        //Init DSound
        /* 
        let hr = dsound::DirectSoundCreate( null_mut(), &mut sound_data.dsound_obj, null_mut());
        println!("Result of DirectSoundCreate {}", hr);
       

        let hr = deref!(sound_data.dsound_obj, IDirectSound).SetCooperativeLevel(window_handle, dsound::DSSCL_PRIORITY);
        println!("Result of SetCooperativeLevel {}", hr);


        //TODO
        //Store the following in app data
        let mut buffer_desc : dsound::DSBUFFERDESC = Default::default();
        buffer_desc.dwSize  = mem::size_of::<dsound::DSBUFFERDESC>() as u32;
        buffer_desc.dwFlags = dsound::DSBCAPS_PRIMARYBUFFER; 

        let mut primary_buffer : dsound::LPDIRECTSOUNDBUFFER = null_mut();

        let hr = deref!(sound_data.dsound_obj, IDirectSound).CreateSoundBuffer( &mut buffer_desc as dsound::LPCDSBUFFERDESC, &mut primary_buffer as *mut dsound::LPDIRECTSOUNDBUFFER, null_mut() );
        println!("Result of CreateSoundBuffer {}", hr);

        let hr = deref!(primary_buffer, dsound::IDirectSoundBuffer).SetFormat( sound_data.audio_prefs );
        println!("Result of SetFormat {}", hr);



        let mut secondary_buffer_desc : dsound::DSBUFFERDESC = Default::default();
        secondary_buffer_desc.dwSize  = mem::size_of::<dsound::DSBUFFERDESC>() as u32;
        secondary_buffer_desc.dwFlags = dsound::DSBCAPS_CTRLPAN | dsound::DSBCAPS_CTRLVOLUME | dsound::DSBCAPS_CTRLFREQUENCY;
        secondary_buffer_desc.dwBufferBytes = 48000; 
        secondary_buffer_desc.lpwfxFormat   = sound_data.audio_prefs; 
        
        let mut secondary_buffer : dsound::LPDIRECTSOUNDBUFFER = null_mut();
        let hr = deref!(sound_data.dsound_obj, IDirectSound).CreateSoundBuffer( &mut secondary_buffer_desc as dsound::LPCDSBUFFERDESC, &mut secondary_buffer as *mut dsound::LPDIRECTSOUNDBUFFER, null_mut() );
        println!("Result of CreateSoundBuffer <Secondary>{} {:?}", hr, secondary_buffer);

 
        let mut playcursor = 0u32;
        let mut writecursor = 0u32;
        let hr = deref!(secondary_buffer, dsound::IDirectSoundBuffer).GetCurrentPosition( &mut playcursor as *mut _, &mut writecursor as *mut _);
        println!("Result of currentposition {} {} {}", hr, playcursor, writecursor);

        let mut region1 = null_mut();
        let mut region1_size = 0u32;
        let mut region2 = null_mut();
        let mut region2_size = 0u32;
        let offset = 0;
        let bytes  = 100;
        let hr = deref!(secondary_buffer, dsound::IDirectSoundBuffer).Lock(
            0,
            0,
            &mut region1 as *mut _, &mut region1_size as *mut _,
            null_mut(), null_mut(),//&mut region2_size as *mut _,
            2
        );
        println!("Result of Lock {} {:?} {}", hr, region1, region1_size);

        {
            //let mut slice = std::slice::from_raw_parts_mut(region1 as *mut i32, 13000 as usize); //region1_size as usize);
            let mut sign = 1;
            for i in 0..13000{//Square wave
              
                if i%100*2 == 0 { sign *=-1; }
                *(region1 as *mut i32).offset(i) = sign*10000;
                //if i%50 ==0 { println!("{} ASDFASDF", i); }
            }
        }
        println!("ASDFASDFADFSADFADFSADFA");

        let hr = deref!(secondary_buffer, dsound::IDirectSoundBuffer).Unlock(
            region1, region1_size,
            region2, region2_size,
        );
        println!("Result of Unlock {}", hr);


        let hr = deref!(secondary_buffer, dsound::IDirectSoundBuffer).Play(0, 0, 0);//dsound::DSBPLAY_LOOPING);
        println!("Result of play {}", hr);

        //let hr = deref!(secondary_buffer, dsound::IDirectSoundBuffer).Stop();
        //println!("Result of stop {}", hr);

        panic!("HELP");
        */
    }

    
    {//Checking  inputs
        if textinfo.character == ' ' && sound_data.record_on == false{
            sound_data.record_on = true; 
            sound_data.to_be_filled = MAX_SOUND_DATA as isize;
        }
        if textinfo.character != ' ' && textinfo.character != '\0'{
            sound_data.button_press_marker = MAX_SOUND_DATA - sound_data.to_be_filled as usize;
        }

        //TODO
        //We really should move gamepad stuff out side of individual apps
        let mut temp_xgamepad = xinput::XINPUT_STATE{
                                    dwPacketNumber: 0,
                                    Gamepad: xinput::XINPUT_GAMEPAD{
                                        wButtons: 0,
                                        bLeftTrigger: 0,
                                        bRightTrigger: 0,
                                        sThumbLX: 0,
                                        sThumbLY: 0,
                                        sThumbRX: 0,
                                        sThumbRY: 0,
                                  }};
        let _temp = xinput::XInputGetState(0, &mut temp_xgamepad as *mut xinput::XINPUT_STATE);
        if temp_xgamepad.Gamepad.wButtons != 0x00 || temp_xgamepad.Gamepad.bRightTrigger > 50{
            sound_data.button_press_marker = MAX_SOUND_DATA - sound_data.to_be_filled as usize;
        } 
    }

    {//Tape deck
        sound_data.textbox.update( keyboardinfo, textinfo, mouseinfo );
        sound_data.textbox.draw( time_instance.subsec_nanos() as f32 );
        
        let x = sound_data.textbox.x;
        let y = sound_data.textbox.y;
        for i in 0..sound_data.tape_deck.len(){
            
            let _i = i as i32 + 1;
            let _y = y - _i*15;
            let offset = drawString(&mut GLOBAL_BACKBUFFER, &format!("{}.  ", _i), x, _y, C4_WHITE, 18.0);
            let length = drawString(&mut GLOBAL_BACKBUFFER, 
                                    &sound_data.tape_deck.keys[i], 
                                    x+offset, _y, C4_WHITE, 18.0);

            if in_rect( mouseinfo.x, mouseinfo.y, [x-1, _y, length+offset+15, 19]) {
                drawRect(&mut GLOBAL_BACKBUFFER,  [x-1, _y, length+offset+15, 19], C4_WHITE, false);
                drawString(&mut GLOBAL_BACKBUFFER, "SAVE", x+length+offset+15, _y, C4_GREY, 19.0);

                if mouseinfo.lbutton == ButtonStatus::Down{
                    drawString(&mut GLOBAL_BACKBUFFER, "SAVE", x+length+offset+15, _y, C4_RED, 19.0);
                }
                
                //SAVE DATA
                if mouseinfo.lbutton == ButtonStatus::Up && mouseinfo.old_lbutton == ButtonStatus::Down{
                    //TODO set directory some other way
                    let mut filename = "training_testing_data/audio/".to_string();
                    filename = filename + &sound_data.tape_deck.keys[i];
                    filename = filename + ".ct";
                    sound_data.tape_deck.values[i].save(&filename);
                }
            }
        }
        //TODO
        //user feed back needed when save is successful
        //drawString(&mut GLOBAL_BACKBUFFER, "SAVED", 15, 15, [1.0, 1.0, 1.0, sound_data.], 32.0);
    } 


    {//Instructions
        drawString(&mut GLOBAL_BACKBUFFER, "Instructions:", 50, 450, C4_YELLOW, 22.0);
        drawString(&mut GLOBAL_BACKBUFFER, "Press space bar to begin recording.", 50, 425, C4_YELLOW, 22.0);
        
    }
    {//TOGGLE record_on
        if sound_data.record_on {
            drawString(&mut GLOBAL_BACKBUFFER, "RECORDING: ON", 350, 400, C4_WHITE, 28.0);
        } else{
            drawString(&mut GLOBAL_BACKBUFFER, "RECORDING: OFF", 350, 400, C4_WHITE, 28.0);
        }
    }

    if sound_data.record_on {
        let mut buffersize = 0u32;
        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).GetBufferSize(&mut buffersize as *mut _);
        assert!(hr==0,"Result of GetBufferSize {}; BufferSize {}", hr, buffersize);

        /*
        println!("\nWaveformat");
        println!("formattag: {:?}",       deref!(sound_data.audio_prefs, WAVEFORMATEX).wFormatTag);
        println!("nChannels: {:?}",       deref!(sound_data.audio_prefs, WAVEFORMATEX).nChannels);
        println!("nSamplesPerSec: {:?}",  deref!(sound_data.audio_prefs, WAVEFORMATEX).nSamplesPerSec);
        println!("BitsPerSample: {:?}",   deref!(sound_data.audio_prefs, WAVEFORMATEX).wBitsPerSample);
        println!("AvgBytesPerSec: {:?}",   deref!(sound_data.audio_prefs, WAVEFORMATEX).nAvgBytesPerSec);
        println!("nBlockAlign: {:?}",     deref!(sound_data.audio_prefs, WAVEFORMATEX).nBlockAlign);
        println!("cbSize: {:?}",          deref!(sound_data.audio_prefs, WAVEFORMATEX).cbSize);
        */


        let hnsActualDuration = REFTIMES_PER_SEC as u64 *
                         buffersize as u64 / deref!(sound_data.audio_prefs, WAVEFORMATEX).nSamplesPerSec as u64;
        //println!("Actual Duration: {}", hnsActualDuration);



        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).Start();
        assert!(hr==0,"could not start audio client {}", hr);
        {
            //Currently we wait about a half a sec to get new data
            std::thread::sleep(time::Duration::from_millis(hnsActualDuration/REFTIMES_PER_MILLISEC/10));


            let mut packetLength = 0u32;
            let hr = deref!(sound_data.captureclient_ptr, IAudioCaptureClient).GetNextPacketSize(&mut packetLength as *mut _);
            assert!(hr==0,"GetNextPacket size run: {}\n Next Packet size {}", hr, packetLength); 

            while packetLength != 0
            {
                let mut pData : *mut u8 = null_mut();
                let mut numFramesAvailable = 0u32;
                let mut flags = 0u32;
                // Get the available data in the shared buffer.
                let hr = deref!(sound_data.captureclient_ptr, IAudioCaptureClient).GetBuffer(
                                       &mut pData as *mut _,
                                       &mut numFramesAvailable as *mut _,
                                       &mut flags as *mut _, null_mut(), null_mut());
                assert!(hr == 0, "AudioCaptureClient GetBuffer goes not work: {}.", hr);

                //Prob wrong
                if (flags & AUDCLNT_BUFFERFLAGS_SILENT == 1)
                {
                    pData = null_mut();  // Tell CopyData to write silence.
                } else {
                    
                    //NOTE
                    //Audio is in 32 bit chuncks as specified by the waveformatex
                    //Additionally waveformatex specifies nchannels == 2
                    let slice = std::slice::from_raw_parts_mut(pData as *mut f32, (numFramesAvailable * 2) as usize);
                    { //Storing sound data

                        let buffer_size = (numFramesAvailable * 2 ) as usize;
                        let record_buffer_size  = sound_data.recorded_buffer.len();
                        let recorded_buffer_ptr = sound_data.recorded_buffer.as_mut_ptr();

                        std::ptr::copy(recorded_buffer_ptr.offset(buffer_size as isize), 
                                       recorded_buffer_ptr, record_buffer_size - buffer_size);

                        std::ptr::copy_nonoverlapping(pData as *mut f32, 
                                  recorded_buffer_ptr.offset((record_buffer_size - buffer_size) as isize), 
                                  buffer_size);
                        sound_data.to_be_filled -= buffer_size as isize;
                    }
                }

                let hr = deref!(sound_data.captureclient_ptr, IAudioCaptureClient).ReleaseBuffer(numFramesAvailable);
                assert!(hr==0,"ReleaseBuffer {}", hr);


                let hr = deref!(sound_data.captureclient_ptr, IAudioCaptureClient).GetNextPacketSize(&mut packetLength as *mut _);
                assert!(hr==0, "Get NextPacketSize {}", hr);

                if sound_data.to_be_filled <= 0 {
                    sound_data.record_on = false;
                    
                    //Add the result to our tapedeck
                    let mut tape_name = sound_data.textbox.text_buffer.clone();

                    let mut is_substring_count = 0;
                    for i in 0..sound_data.tape_deck.len(){
                        if sound_data.tape_deck.keys[i].contains(&tape_name) {
                            is_substring_count += 1;
                        }
                    }



                    if is_substring_count > 0 {
                        tape_name += &format!("_{}", is_substring_count);
                    }
                   
                    let mut tape = CassetteTape::new();
                    tape.begin_marker = sound_data.button_press_marker;
                    tape.data =  sound_data.recorded_buffer.clone();

                    sound_data.tape_deck.insert_or_set(tape_name, tape);
                    break;
                }
            }
        }
        let hr = deref!(sound_data.audioclient_ptr, IAudioClient).Stop();
        assert!(hr==0,"stop recording audio {}", hr);

        if sound_data.record_on == false{
            println!("Reset testing");
            let hr = deref!(sound_data.audioclient_ptr, IAudioClient).Reset();
            assert!(hr==0,"Reset recording audio {}", hr);
        }
    }
    { //Draw recording
        let fw = 0.4;
        let fh = 0.4;
        let offset_x =  (GLOBAL_BACKBUFFER.w as f32 * (1.0-fw-0.03)) as i32;
        let offset_y =  (GLOBAL_BACKBUFFER.h as f32 * fh) as i32; 
        let w        =  (GLOBAL_BACKBUFFER.w as f32 * fw) as i32;
        let h        =  (GLOBAL_BACKBUFFER.h as f32 * fh) as i32; 

        drawRect(&mut GLOBAL_BACKBUFFER, [offset_x, offset_y, w, h], C4_GREY, true);


        let mut _i = 0;
        for i in 0..sound_data.recorded_buffer.len(){
            if i % 1000 != 0 { continue; }
            _i +=1;
            let x = (_i) as i32 + offset_x;
            let hf32 = h as f32;
            let y = (sound_data.recorded_buffer[i] * hf32 + 0.5*hf32) as i32 + offset_y;
            let w = 1;
            let h = 1;
            drawRect(&mut GLOBAL_BACKBUFFER, [x, y, w, h], C4_BLUE, true);
        }

        //Draw marker is set

        //TODO 
        //make this nice
        let x = (sound_data.button_press_marker/2000*2) as i32 + offset_x;
        //println!("{}", x);
        let y = offset_y - h/2;
        drawRect(&mut GLOBAL_BACKBUFFER, [x, offset_y, 2, h], C4_RED, true);
    }

    return 0;
}}



fn app_rectangle(app_data: &mut AppData, keyboardinfo: &KeyboardInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration)->i32{unsafe{
    drawRect(&mut GLOBAL_BACKBUFFER, [0, 0, GLOBAL_BACKBUFFER.w, GLOBAL_BACKBUFFER.h], [0.2, 0.2, 0.2, 1.0], true);
    drawString(&mut GLOBAL_BACKBUFFER, "Something about a rectangular sailor", 350, 450, [1.0, 1.0, 1.0, 1.0], 34.0);

    drawString(&mut GLOBAL_BACKBUFFER, "Directions: 'a' and 'd' to cycle images", 5, 100,    C4_YELLOW, 20.0);
    drawString(&mut GLOBAL_BACKBUFFER, "Directions: mouse wheel to cycle rectangles", 5, 80, C4_YELLOW, 20.0);
    drawString(&mut GLOBAL_BACKBUFFER, "Directions: SPACEBAR to save results ", 5, 60,       C4_YELLOW, 20.0);
    drawString(&mut GLOBAL_BACKBUFFER, "Directions: 'right click' to place vrts", 5, 40,     C4_YELLOW, 20.0);
    //TODO
    //usage instructions


    let rectapp_data = &mut app_data.rect_data;
    if !rectapp_data.init{
        rectapp_data.init = true;
        ///////////////////
        //Setup TextBox
        rectapp_data.folder_path_textbox.x = 250;
        rectapp_data.folder_path_textbox.y = 10;
        rectapp_data.folder_path_textbox.max_char = 80;
        rectapp_data.folder_path_textbox.max_render_length = 600;
        rectapp_data.folder_path_textbox.text_color = [ 0.8, 0.8, 0.8, 0.7];
        rectapp_data.folder_path_textbox.text_buffer = "temp".to_string();
    }
    {//Iterate to a new BMP
        //TODO
        //store results so far and reset _rects and rects
        if textinfo.character == 'd'{
            rectapp_data.nth_file += 1;
            rectapp_data.active_bmp_init = false;
        } else if textinfo.character == 'a' {
            if rectapp_data.nth_file > 0 { rectapp_data.nth_file -= 1; }
            rectapp_data.active_bmp_init = false;
        }
    }
    {//TextBox stuffs
        rectapp_data.folder_path_textbox.update(keyboardinfo, textinfo, mouseinfo);
        drawString(&mut GLOBAL_BACKBUFFER, "Path: ", 185, 8, [0.8, 0.8, 0.8, 1.0], 32.0);
        let _path = rectapp_data.folder_path_textbox.text_buffer.clone();
        let mut _path = std::path::Path::new(&_path);
        if _path.exists() &&
           _path.is_dir() {

            let mut ith_bmp = 0;
            for entry in std::fs::read_dir(_path).unwrap(){
                let entry = entry.expect("Trying to read dir for bmps");

                if entry.path().is_file(){
                    let _p = entry.path();
                    if entry.path().extension().unwrap() == "bmp"{
                        if rectapp_data.nth_file == ith_bmp &&
                           rectapp_data.active_bmp_init == false{

                            /////////////////////////////////
                            //Resetting data
                            rectapp_data.nth_player = 0;
                            if rectapp_data.active_bmp_name.len() > 0 {
                                let _name = rectapp_data.active_bmp_name.clone();
                                rectapp_data.store(&_name);
                                rectapp_data._rects = [[0;4];10];
                                rectapp_data.rects = [[0;4];10];
                            }
                            /////////////////////////////////

                            rectapp_data.active_bmp_init = true;
                            rectapp_data.active_bmp_name = _p.to_str().unwrap().to_string();
                            let _bmp = loadBMP(&rectapp_data.active_bmp_name);
                            rectapp_data.temp_bmp_w_h = [_bmp.info_header.width, _bmp.info_header.height];
                            let w = rectapp_data.bmp_box[2];
                            let h = rectapp_data.bmp_box[3];
                            rectapp_data.active_bmp =  resizeBMP( &_bmp, w, h);

                            /////////////////////////////////
                            //Setting stored data
                            let _name = rectapp_data.active_bmp_name.clone();
                            rectapp_data.set_rects(&_name);
                            /////////////////////////////////


                            break;
                        }
                        ith_bmp += 1;
                    }
                }

            }
        }
        rectapp_data.folder_path_textbox.draw(time_instance.subsec_nanos() as f32);
    }

    //Draws active bmp, associated outline rect
    {
        drawBMP(&mut GLOBAL_BACKBUFFER, &rectapp_data.active_bmp, rectapp_data.bmp_box[0],
             rectapp_data.bmp_box[1], 1.0, None, None);
        drawRect(&mut GLOBAL_BACKBUFFER, rectapp_data.bmp_box, [1.0;4], false);
        drawString(&mut GLOBAL_BACKBUFFER, &rectapp_data.active_bmp_name, 400, 30, [0.0, 1.0, 0.0, 0.75], 20.0);
    }
    {//Update active "player" rect
        let mut index = rectapp_data.nth_player as i32 - mouseinfo.wheel_delta ;
        if index > 9 {
            index = 0;
        } else if index < 0{
            index = 9;
        }
        rectapp_data.nth_player = index as usize;
    }

    let mut nth_player = rectapp_data.nth_player;
    if in_rect(mouseinfo.x, mouseinfo.y, rectapp_data.bmp_box){//User draws rect
        if mouseinfo.old_lbutton == ButtonStatus::Down && mouseinfo.lbutton == ButtonStatus::Up{
            if rectapp_data.xy_or_wh {
                rectapp_data.active = true;
                rectapp_data.rects[nth_player][0] = mouseinfo.x;
                rectapp_data.rects[nth_player][1] = mouseinfo.y;
                rectapp_data._temp_rect[0] = mouseinfo.x;
                rectapp_data._temp_rect[1] = mouseinfo.y;
            } else{
                rectapp_data.active = false;
                rectapp_data._temp_rect = [0i32;4];
                rectapp_data.nth_player += 1;
            }
            rectapp_data.xy_or_wh = !rectapp_data.xy_or_wh;
        }
        if rectapp_data.active{
            let mut _xywh = rectapp_data._temp_rect.clone();
            _xywh[2] = (rectapp_data._temp_rect[0] - mouseinfo.x).abs();
            _xywh[3] = (rectapp_data._temp_rect[1] - mouseinfo.y).abs();
            if mouseinfo.x < rectapp_data._temp_rect[0]{
                _xywh[0] = mouseinfo.x;
            }
            if mouseinfo.y < rectapp_data._temp_rect[1]{
                _xywh[1] = mouseinfo.y;
            }
            drawRect(&mut GLOBAL_BACKBUFFER, _xywh, [1.0, 1.0, 1.0, 1.0], false);
            rectapp_data.rects[nth_player] = _xywh;
        }
    }
    {//convert rect to _rect with bmp coordinates
        #[inline]
        fn convert_rect(input: [i32;4], original_w: i32, original_h: i32, post_w: i32, post_h: i32, offset_x: i32, offset_y: i32)->[i32;4]{
            let mut rt = [0;4];
            let o_w = original_w as f32;
            let o_h = original_h as f32;

            let p_w = post_w as f32;
            let p_h = post_h as f32;

            rt[0] = if input[0] !=0 { ((input[0] - offset_x) as f32 * o_w/p_w) as i32 }  else { 0 };
            rt[1] = if input[1] != 0 {((input[1] - offset_y) as f32 * o_h/p_h) as i32 } else { 0 };
            rt[2] = (input[2] as f32 * o_w/p_w) as i32;
            rt[3] = (input[3] as f32 * o_h/p_h) as i32;
            return rt;
        }
        for (i,it) in rectapp_data.rects.iter().enumerate(){
            rectapp_data._rects[i] = convert_rect(*it, rectapp_data.temp_bmp_w_h[0], rectapp_data.temp_bmp_w_h[1],
                                                        rectapp_data.bmp_box[2], rectapp_data.bmp_box[3],
                                                        rectapp_data.bmp_box[0], rectapp_data.bmp_box[1]);
        }
    }
    //Draw rect lables
    for i in 0..rectapp_data._rects.len() {
        let _i = i as i32;
        let mut color = [1.0f32; 4];
        if rectapp_data.nth_player == i {
            color[0] = 0.0;
        }
        drawString(&mut GLOBAL_BACKBUFFER, &format!("{:?} {:?}", i, &rectapp_data._rects[i]) ,10, 400 - _i*23, color, 24.0);
        if i == 0 {
            drawString(&mut GLOBAL_BACKBUFFER, "Lp" ,200, 400 - _i*23, [1.0, 0.0, 0.0, 1.0], 24.0);
        } else if i == 1{
            drawString(&mut GLOBAL_BACKBUFFER, "RP" ,200, 400 - _i*23, [1.0, 0.0, 0.0, 1.0], 24.0);
        }
    }
    //Draw all stable rects
    for i in 0..rectapp_data.rects.len(){
        if rectapp_data.nth_player == i { continue; }
        drawRect(&mut GLOBAL_BACKBUFFER, rectapp_data.rects[i], [0.0, 1.0, 1.0, 0.7], false);
    }
    //Save results
    if textinfo.character == ' '{
        let _name = rectapp_data.active_bmp_name.clone();
        rectapp_data.store(&_name);
        rectapp_data.write("TEST_TESTING");
        rectapp_data.are_we_writing = 300;
    }
    if rectapp_data.are_we_writing >= 0 {
        rectapp_data.are_we_writing -=1;
        let _alpha  = rectapp_data.are_we_writing as f32 / 300.0;

        drawString(&mut GLOBAL_BACKBUFFER, "Writing file : TEST_TESTING.txt", 23, 0, [0.5, 0.8, 0.5, _alpha], 20.0);
        drawRect(&mut GLOBAL_BACKBUFFER, [0, 5, 20, 20], [0.0, 1.0, 0.0, _alpha], true);
    }


    drawString(&mut GLOBAL_BACKBUFFER, &format!("{:?}", rectapp_data.xy_or_wh) ,10, 170, [1.0, 1.0, 1.0, 1.0], 24.0);
    return 0;
}}

fn app_ai(app_data: &mut AppData, keyboardinfo: &KeyboardInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration)->i32{unsafe{
    //TODO
    //Breaks when you exit

    let mut ai_data = &mut app_data.ai_data;
    if ai_data.init == false{
        ai_data.init = true;
        ai_data.glyph_model.load_graph_from_file("assets/ai_models/glyph_NN.pb", None).expect("model load failed");
        //println!("{:?}", ai_data.glyph_model.get_input_dimensions());
        //println!("{:?}", ai_data.glyph_model.get_output_dimensions());
        ai_data.character_portrait_model.load_graph_from_file("assets/ai_models/character_model.pb", None).expect("model load failed");
        println!("{:?}", ai_data.character_portrait_model.get_input_dimensions());
        println!("{:?}", ai_data.character_portrait_model.get_output_dimensions());
        {//Setting up character portrait model result map
            use std::io::BufReader;
            let mut f = File::open("assets/ai_models/character_model_key.txt").expect("model key failed");
            for _line in BufReader::new(f).lines(){
                let line = _line.expect("could not read string");
                let buffer : Vec<&str> = line.split(' ').collect();
                ai_data.character_portrait_map.insert_or_set(buffer[1].parse::<i32>().unwrap(), buffer[0].to_string());
            }
        }
    }
    //NOTE
    //Just coloring the screen somthing special no biggy
    drawRect(&mut GLOBAL_BACKBUFFER, [0, 0, GLOBAL_BACKBUFFER.w, GLOBAL_BACKBUFFER.h], [0.25, 0.21, 0.2, 1.0], true);

    if !app_data.handle_dc.is_some(){
        //TODO
        //This may need to be specific or something we are using info prevy to a different app
        //Might also what to allow the user to set this  maybe in the future since you might just
        // move the the screencaputer app through the menu.  That is a bit of a pain, but this is
        // an upgrade for the future
        if foundWindow(&app_data.capture_exe_textbox.text_buffer){
            app_data.handle_dc = Some(load_handle_dc(&app_data.capture_exe_textbox.text_buffer));
        }
    }
    else {

        let screen = screen_shot(app_data.handle_dc.as_ref().expect("App data dc could not be taken as ref"),
                                1, "", "", false);


        fn get_glyph_bmp_data(glyph_window: &[usize], intensity_buffer: &mut Vec::<f32>,
                            sum_col_int: &mut Vec::<f32>, sum_row_int: &mut Vec::<f32>,
                            screen: &[TGBitmap], debug_coord: &[i32], draw_debug: bool){unsafe{
            //TODO
            //Clap on screen height and width
            for j in 0..glyph_window[3]{//Iterate over the height
                for i in 0..glyph_window[2]{//Iterate over the width
                    let x = 4 * (i + glyph_window[0]);
                    let y = 4 * (glyph_window[1] + j);

                    let r = screen[0].rgba[x + y * screen[0].info_header.width as usize + 2] as f32 / 255.;
                    let g = screen[0].rgba[x + y * screen[0].info_header.width as usize + 1] as f32 / 255.;
                    let b = screen[0].rgba[x + y * screen[0].info_header.width as usize + 0] as f32 / 255.;
                    let a = screen[0].rgba[x + y * screen[0].info_header.width as usize + 3] as f32 / 255.;

                    let intensity = (r + g + b) / 3.0;
                    intensity_buffer.push(intensity.powf(2.0));
                    sum_col_int[i] += intensity;
                    sum_row_int[j] += intensity;

                    if draw_debug{//draw enhanced player name
                        let _x = debug_coord[0];
                        let _y = debug_coord[1] - 60;
                        drawRect(&mut GLOBAL_BACKBUFFER, [_x + 2*i as i32, _y + 2*j as i32, 2, 2], [r, g, b, 1.0], true);
                    }
                }
            }
        }}
        let p1_coor_glyph_ai = [20, 400];
        let p1_text_window = [ 150, 749-95-23, 150, 23];
        let mut p1_intensity_buffer = Vec::<f32>::with_capacity(p1_text_window[2] * p1_text_window[3]);
        let mut p1_glyph_locations = Vec::<usize>::new();
        let mut p1_sum_row_int = vec![0.0f32; p1_text_window[3]];
        let mut p1_sum_col_int = vec![0.0f32; p1_text_window[2]];
        get_glyph_bmp_data(&p1_text_window, &mut p1_intensity_buffer,
                           &mut p1_sum_col_int, &mut p1_sum_row_int,
                           &screen, &p1_coor_glyph_ai, ai_data.glyph_diagnostics_render);

        struct LocGlyphSettings{
            min_abs: f32,
            min_sum_intensity: f32,
            min_width: i32,
        }

        fn find_glyphs(sum_col_int: &Vec<f32>, glyph_brackets: &mut Vec<usize>, settings: &LocGlyphSettings){//finding the glyph brackets
            let _len = sum_col_int.len();
            for (i, it) in sum_col_int[0.._len-3].iter().enumerate(){
                let _abs = ((sum_col_int[i+1] - it) + (sum_col_int[i+2] - sum_col_int[i+1])).abs();
                if _abs > settings.min_abs && sum_col_int[i+1] < settings.min_sum_intensity{
                    glyph_brackets.push(i+1);
                }
            }
            let mut _pop = vec![];
            for i in 0..glyph_brackets.len() - 1{
                if (glyph_brackets[i] as i32 - glyph_brackets[i+1] as i32).abs() < settings.min_width{
                    _pop.push(i)
                }
            }
            for (i, it) in _pop.iter().enumerate(){
                glyph_brackets.remove(it - i);
            }
        }
        let settings = LocGlyphSettings{
            min_abs: 2.5,
            min_sum_intensity: 15.0,
            min_width: 4,
        };
        let mut p1_glyph_brackets = vec![];
        find_glyphs(&p1_sum_col_int, &mut p1_glyph_brackets, &settings);

        let p2_coor_glyph_ai = [20, 220];
        let p2_text_window = [ 980, 749-95-23, 150, 23];//TODO Load of settings file
        let mut p2_intensity_buffer = Vec::<f32>::with_capacity(p2_text_window[2] * p2_text_window[3]);
        let mut p2_glyph_locations = Vec::<usize>::new();
        let mut p2_sum_row_int = vec![0.0f32; p2_text_window[3]];
        let mut p2_sum_col_int = vec![0.0f32; p2_text_window[2]];
        get_glyph_bmp_data(&p2_text_window, &mut p2_intensity_buffer,
                           &mut p2_sum_col_int, &mut p2_sum_row_int,
                            &screen, &p2_coor_glyph_ai, ai_data.glyph_diagnostics_render);

        let mut p2_glyph_brackets = vec![];
        find_glyphs(&p2_sum_col_int, &mut p2_glyph_brackets, &settings);

        //NOTE
        //this is debug material
        if ai_data.glyph_diagnostics_render {//Draw glyph debug info
            //TODO
            //Clean this shit up all these damn offsets and shit .... :(
            //Might not want these things hard coded... idk maybe we do.
            let _yoffset = -180;
            drawRect(&mut GLOBAL_BACKBUFFER, [p1_coor_glyph_ai[0]-2, p1_coor_glyph_ai[1] + _yoffset+30, 304, 110-30],
                                            [0.3, 0.3, 0.3, 1.0], true);
            for (i,it) in p1_sum_col_int.iter().enumerate(){//Draws the curve
                let _x = 2*i as i32 + p1_coor_glyph_ai[0];
                let _y = (*it * 5.0) as i32 + p1_coor_glyph_ai[1] + _yoffset;
                drawRect(&mut GLOBAL_BACKBUFFER, [_x, _y, 2, 2], [0.1, 0.6, 0.6, 1.0], true);
            }
            for it in p1_glyph_brackets.iter(){//Draws the glyph brackets
                let _x = 2*(*it) as i32 + p1_coor_glyph_ai[0];
                for j in 0..16{
                    let _y = (j as f32 * 5.0) as i32 + p1_coor_glyph_ai[1] - 150;
                    drawRect(&mut GLOBAL_BACKBUFFER, [_x, _y, 2, 3], [0.9, 0.9, 0.9, 0.6], true);
                }
            }

            drawRect(&mut GLOBAL_BACKBUFFER, [p2_coor_glyph_ai[0]-2, p2_coor_glyph_ai[1] + _yoffset+30, 304, 110-30],
                                            [0.3, 0.3, 0.3, 1.0], true);
            for (i,it) in p2_sum_col_int.iter().enumerate(){//Draws the curve
                let _x = 2*i as i32 + p2_coor_glyph_ai[0];
                let _y = (*it * 5.0) as i32 + p2_coor_glyph_ai[1] + _yoffset;
                drawRect(&mut GLOBAL_BACKBUFFER, [_x, _y, 2, 2], [0.1, 0.6, 0.6, 1.0], true);
            }
            for it in p2_glyph_brackets.iter(){//Draws the glyph brackets
                let _x = 2*(*it) as i32 + p2_coor_glyph_ai[0];
                for j in 0..16{
                    let _y = (j as f32 * 5.0) as i32 + p2_coor_glyph_ai[1] - 150;
                    drawRect(&mut GLOBAL_BACKBUFFER, [_x, _y, 2, 3], [0.9, 0.9, 0.9, 0.6], true);
                }
            }
        }

        for (glyph_i, glyph_iter) in p1_glyph_brackets.iter().enumerate(){
            if glyph_i == 0 { continue; }
            let mut arr = vec![0.0f32; 1*28*28*1];
            let glyph_pos1 = p1_glyph_brackets[glyph_i - 1];
            // the two is an offset used in responce to a defect in the glyph detection algo
            //should be be removed and reaplaced by a better algo
            let glyph_pos2 = *glyph_iter;

            //CLEANUP
            //We had to flip the letter here it was a pain in the ass and this clode block needs be cleaned up
            //We also want to render all of the glyphs associated with the character name

            //TODO finish making this a function
            //fn predict_and_debug(player_text_window: &[i32], player_intensity_buffer: &[f32])
            {
                let mut cursor = 0;
                //I think this is a reflection about both x and y
                //It might be a good idea to explore all the effects of rev on 2d matrix indices
                for (i, it) in (0..20).rev().enumerate(){
                    //NOTE
                    //The minus one is to correct for something in the find glyph algo
                    for (j, jt) in ((glyph_pos1 - 1)..glyph_pos2).rev().enumerate(){
                        let _temp = ((28 - (glyph_pos2 - (glyph_pos1-1))) as f32 / 2.0 ) as usize;
                        cursor = (i+3)*28 + (j + _temp) ;
                        let _cursor = it*p1_text_window[2] + jt;
                        arr[cursor] = p1_intensity_buffer[_cursor];
                    }
                }
                //NOTE
                //More debugging rendering
                if ai_data.glyph_diagnostics_render{
                    let _offsetx = p1_coor_glyph_ai[0]+30 + 35 * (glyph_i as i32 - 1);
                    let _offsety = p1_coor_glyph_ai[1]-170;
                    for i in (0..28){
                        for j in (0..28){
                            let _r = arr[(27-i) + (27-j)*28];
                            drawRect(&mut GLOBAL_BACKBUFFER, [i as i32 + _offsetx, j as i32 + _offsety, 1, 1], [_r, _r, _r, 1.0], true);
                        }
                    }
                }
            }
            let mut _max_arg = 0;
            let mut _max = 0.0;

            for (i, it) in ai_data.glyph_model.predict(&mut arr, 1).unwrap().iter().enumerate(){
                if *it > _max{
                    _max_arg = i;
                    _max = *it;
                }
            }

            //let _offsetx = p1_coor_glyph_ai[0]+30 + 35 * (glyph_i as i32 - 1);
            let _offsety = if ai_data.glyph_diagnostics_render {p1_coor_glyph_ai[1]-190} else {p1_coor_glyph_ai[1] - 20};
            if _max_arg >= 10{
                let str_pred = format!("{} {:.2} ", std::char::from_u32( _max_arg as u32 + 55).unwrap(), _max);
                drawString(&mut GLOBAL_BACKBUFFER, &str_pred, 45*glyph_i as i32 - 35, _offsety, [1.0, 1.0, 1.0, 1.0], 20.0);
            }
            else {
                let str_pred = format!("{} {:.2} ", _max_arg, _max);
                drawString(&mut GLOBAL_BACKBUFFER, &str_pred, 45*glyph_i as i32 - 35, _offsety, [1.0, 1.0, 1.0, 1.0], 20.0);
            }
        }
        for (glyph_i, glyph_iter) in p2_glyph_brackets.iter().enumerate(){
            if glyph_i == 0 { continue; }
            let mut arr = vec![0.0f32; 1*28*28*1];
            let glyph_pos1 = p2_glyph_brackets[glyph_i - 1];
            let glyph_pos2 = *glyph_iter;

            //CLEANUP
            //We had to flip the letter here it was a pain in the ass and this clode block needs be cleaned up
            //We also want to render all of the glyphs associated with the character name

            //TODO finish making this a function
            //fn predict_and_debug(player_text_window: &[i32], player_intensity_buffer: &[f32])
            {
                let mut cursor = 0;
                //I think this is a reflection about both x and y
                //It might be a good idea to explore all the effects of rev on 2d matrix indices
                for (i, it) in (0..20).rev().enumerate(){
                    for (j, jt) in (glyph_pos1..glyph_pos2).rev().enumerate(){
                        let _temp = ((28 - (glyph_pos2 - glyph_pos1)) as f32 / 2.0 ) as usize;
                        cursor = (i+3)*28 + (j + _temp) ;
                        let _cursor = it*p2_text_window[2] + jt;
                        arr[cursor] = p2_intensity_buffer[_cursor];
                    }
                }
                //NOTE
                //More debugging rendering
                if ai_data.glyph_diagnostics_render{
                    let _offsetx = p2_coor_glyph_ai[0]+30 + 35 * (glyph_i as i32 - 1);
                    let _offsety = p2_coor_glyph_ai[1]-170;
                    for i in (0..28){
                        for j in (0..28){
                            let _r = arr[(27-i) + (27-j)*28];
                            drawRect(&mut GLOBAL_BACKBUFFER, [i as i32 + _offsetx, j as i32 + _offsety, 1, 1], [_r, _r, _r, 1.0], true);
                        }
                    }
                }
            }
            let mut _max_arg = 0;
            let mut _max = 0.0;

            //Determine index of best prediction
            for (i, it) in ai_data.glyph_model.predict(&mut arr, 1).unwrap().iter().enumerate(){
                if *it > _max{
                    _max_arg = i;
                    _max = *it;
                }
            }

            //Draw results of the prediction
            let _offsety = if ai_data.glyph_diagnostics_render {p2_coor_glyph_ai[1]-190 } else { p1_coor_glyph_ai[1] - 40 };
            if _max_arg >= 10{
                let str_pred = format!("{} {:.2} ", std::char::from_u32( _max_arg as u32 + 55).unwrap(), _max);
                drawString(&mut GLOBAL_BACKBUFFER, &str_pred, 45*glyph_i as i32 - 35, _offsety, [1.0, 1.0, 1.0, 1.0], 20.0);
            }
            else {
                let str_pred = format!("{} {:.2} ", _max_arg, _max);
                drawString(&mut GLOBAL_BACKBUFFER, &str_pred, 45*glyph_i as i32 - 35, _offsety, [1.0, 1.0, 1.0, 1.0], 20.0);
            }

        }
        //////////////////////
        //Health and meters
        //
        {

            let health_present_rgb = [0xff, 0xbb, 0x21];
            let health_present_rgb_delta = [100, 100, 40];

            let health_absent_rgb = [0, 0, 0];
            let health_absent_rgb_delta = [15, 15, 15];

            let health_change_rgb = [0x87, 0, 0];
            let health_change_rgb_delta = [15, 15, 15];


            //TODO
            //COMPLETE ME
            //let _v = (height - 80) * width + 150;
            fn determine_health( bmp: &TGBitmap, x: usize, y: usize, w:usize, health_present_rgb: [i32; 3], health_present_rgb_delta: [i32; 3],
                                                                     health_absent_rgb: [i32; 3], health_absent_rgb_delta: [i32; 3],
                                                                     health_change_rgb: [i32; 3], health_change_rgb_delta: [i32; 3], debug: bool
            )->(f32, f32, f32){
                let mut percent_health_present = 0.0f32;
                let mut percent_health_absent = 0.0f32;
                let mut percent_health_change = 0.0f32;

                let width  = bmp.info_header.width as usize;
                let height = bmp.info_header.height as usize;
                let _v = (height - y) * width + x;
                let mut _i = 0;
                for i in _v .. _v+w{
                    let r  = bmp.rgba[4*i+2] as f32 / 255.0;
                    let g  = bmp.rgba[4*i+1] as f32 / 255.0;
                    let b  = bmp.rgba[4*i+0] as f32 / 255.0;
                    let _r = bmp.rgba[4*i+2] as i32 ;
                    let _g = bmp.rgba[4*i+1] as i32 ;
                    let _b = bmp.rgba[4*i+0] as i32 ;

                    {//health present
                        let mut pass_health_present = true;
                        if (_r - health_present_rgb[0]).abs() > health_present_rgb_delta[0]{
                            pass_health_present = false;
                        }
                        if (_g - health_present_rgb[1]).abs() > health_present_rgb_delta[1]{
                            pass_health_present = false;
                        }
                        if (_b - health_present_rgb[2]).abs() > health_present_rgb_delta[2]{
                            pass_health_present = false;
                        }
                        if pass_health_present{
                            percent_health_present += 1.0;
                        } else{
                            //println!("{:?} {:?}", &[_r, _g, _b], &health_present_rgb);
                        }
                    }
                    {//health absent
                        let mut pass_health_absent = true;
                        if (_r - health_absent_rgb[0]).abs() > health_absent_rgb_delta[0]{
                            pass_health_absent = false;
                        }
                        if (_g - health_absent_rgb[1]).abs() > health_absent_rgb_delta[1]{
                            pass_health_absent = false;
                        }
                        if (_b - health_absent_rgb[2]).abs() > health_absent_rgb_delta[2]{
                            pass_health_absent = false;
                        }
                        if pass_health_absent{
                            percent_health_absent += 1.0;
                        } else{
                            //println!("{:?} {:?}", &[_r, _g, _b], &health_present_rgb);
                        }
                    }
                    {//health delta
                        let mut pass_health_change = true;
                        if (_r - health_change_rgb[0]).abs() > health_change_rgb_delta[0]{
                            pass_health_change = false;
                        }
                        if (_g - health_change_rgb[1]).abs() > health_change_rgb_delta[1]{
                            pass_health_change = false;
                        }
                        if (_b - health_change_rgb[2]).abs() > health_change_rgb_delta[2]{
                            pass_health_change = false;
                        }
                        if pass_health_change{
                            percent_health_change += 1.0;
                        } else{
                            //println!("{:?} {:?}", &[_r, _g, _b], &health_present_rgb);
                        }
                    }
                    //FOR DEBUG
                    if debug{
                        unsafe{ drawRect(&mut GLOBAL_BACKBUFFER, [0+2*_i as i32 ,0, 2, 2], [r, g, b, 1.0], true); }
                    }
                    _i += 1;
                }

                percent_health_present /= w as f32;
                percent_health_absent  /= w as f32;
                percent_health_change  /= w as f32;
                //TODO
                //drawString ME
                return (percent_health_present, percent_health_absent, percent_health_change);
            }
            //player 1
            let (p1_health_present, p1_health_absent, p1_health_change) = determine_health( &screen[0], 150, 80, 400, health_present_rgb, health_present_rgb_delta,
                                                health_absent_rgb, health_absent_rgb_delta,
                                                health_change_rgb, health_change_rgb_delta, false);
            let (p2_health_present, p2_health_absent, p2_health_change) = determine_health( &screen[0], 730, 80, 400, health_present_rgb, health_present_rgb_delta,
                                                                    health_absent_rgb, health_absent_rgb_delta,
                                                                    health_change_rgb, health_change_rgb_delta, false);

            //TODO render to some proper place
            if !ai_data.glyph_diagnostics_render {
                drawString(&mut GLOBAL_BACKBUFFER, &format!("{} {} {}", p1_health_present, p1_health_absent, p1_health_change), 10, 70, [1.0, 1.0, 1.0, 1.0], 20.0);
                drawString(&mut GLOBAL_BACKBUFFER, &format!("{} {} {}", p2_health_present, p2_health_absent, p2_health_change), 10, 50, [1.0, 1.0, 1.0, 1.0], 20.0);
            }
            //TODO
            //make function specific to getting meter
            //Getting meter info
            let meter_present_rgb = [0xbb, 0xbb, 0xbb];
            let meter_present_rgb_delta = [100, 100, 100];

            let meter_absent_rgb = [0, 0, 0];
            let meter_absent_rgb_delta = [15, 15, 15];

            let meter_change_rgb = [0x87, 10, 10];
            let meter_change_rgb_delta = [1, 1, 1];
            let (p1_meter_present, p1_meter_absent, p1_meter_change) = determine_health( &screen[0], 80, 700, 230, meter_present_rgb, meter_present_rgb_delta,
                                                         meter_absent_rgb,  meter_absent_rgb_delta,
                                                         meter_change_rgb,  meter_change_rgb_delta, false);
            let (p2_meter_present, p2_meter_absent, p2_meter_change) = determine_health( &screen[0], 970, 700, 230, meter_present_rgb, meter_present_rgb_delta,
                                                         meter_absent_rgb,  meter_absent_rgb_delta,
                                                         meter_change_rgb,  meter_change_rgb_delta, true);
            //TODO render to some proper place
            if !ai_data.glyph_diagnostics_render {
                drawString(&mut GLOBAL_BACKBUFFER, &format!("{} {} {}", p1_meter_present, p1_meter_absent, p1_meter_change), 10, 30, [1.0, 1.0, 1.0, 1.0], 20.0);
                drawString(&mut GLOBAL_BACKBUFFER, &format!("{} {} {}", p2_meter_present, p2_meter_absent, p2_meter_change), 10, 10, [1.0, 1.0, 1.0, 1.0], 20.0);
            }
        }
        {//character portrait identification
            //NOTE These portraits are only applicable to GGxrd
            let p1_portrait = [40, 25, 98, 98];
            let p2_portrait = [40, 25, 98, 98];


            //Player 1 portrait
            {//FLIP portrait
                let mut arr_p1 = [0.0f32; 98*98*3];
                {//Get stuff for player 1
                    let mut cursor = 0;
                    let x1 = p1_portrait[0];
                    let x2 = x1 + p1_portrait[2];
                    let y = p1_portrait[1];
                    //I think this is a reflection about both x and y
                    //It might be a good idea to explore all the effects of rev on 2d matrix indices
                    for (i, it) in (0..p1_portrait[3]).enumerate(){
                        for (j, jt) in (x1..x2).rev().enumerate(){//this might be a y? Not an x...
                            //TODO
                            //Clamp ME PLZ

                            cursor = (i*p1_portrait[3] + j) * 3;
                            let _cursor = 4*((screen[0].info_header.height as usize - it - y)*screen[0].info_header.width as usize + jt);
                            arr_p1[cursor + 0] = screen[0].rgba[_cursor + 2] as f32 / 255.0; //R
                            arr_p1[cursor + 1] = screen[0].rgba[_cursor + 1] as f32 / 255.0; //G
                            arr_p1[cursor + 2] = screen[0].rgba[_cursor + 0] as f32 / 255.0; //B
                        }
                    }
                }
                let mut arr_p2 = [0.0f32; 98*98*3];
                {//Get stuff for player 2
                    let mut cursor = 0;
                    let x1 = p2_portrait[0];
                    let x2 = x1 + p2_portrait[2];
                    let y = p2_portrait[1];
                    //I think this is a reflection about both x and y
                    //It might be a good idea to explore all the effects of rev on 2d matrix indices
                    for (i, it) in (0..p2_portrait[3]).enumerate(){
                        for (j, jt) in (x1..x2).rev().enumerate(){//this might be a y? Not an x...
                            //TODO
                            //Clamp ME PLZ

                            cursor = (i*p2_portrait[3] + j) * 3;
                            let _cursor = 4*((screen[0].info_header.height as usize - it - y)*screen[0].info_header.width as usize + jt);
                            arr_p2[cursor + 0] = screen[0].rgba[_cursor + 2] as f32 / 255.0; //R
                            arr_p2[cursor + 1] = screen[0].rgba[_cursor + 1] as f32 / 255.0; //G
                            arr_p2[cursor + 2] = screen[0].rgba[_cursor + 0] as f32 / 255.0; //B
                        }
                    }
                }

                let mut p1_max_arg = 0;
                let mut p1_max = 0.0;
                {//Predict player 1 portrait
                    let mut p1_max_arg = 0;
                    let mut p1_max = 0.0;
                    let prediction = ai_data.character_portrait_model.predict(&mut arr_p1, 1).unwrap();
                    for (i, it) in prediction.iter().enumerate(){
                        if *it > p1_max{
                            p1_max_arg = i;
                            p1_max = *it;
                        }
                    }
                }
                let mut p2_max_arg = 0;
                let mut p2_max = 0.0;
                {//Predict player 1 portrait
                    let mut p2_max_arg = 0;
                    let mut p2_max = 0.0;
                    let prediction = ai_data.character_portrait_model.predict(&mut arr_p2, 1).unwrap();
                    for (i, it) in prediction.iter().enumerate(){
                        if *it > p2_max{
                            p2_max_arg = i;
                            p2_max = *it;
                        }
                    }
                }
                println!("Player 1 : {} {}", p1_max_arg, p1_max);
                println!("{}",ai_data.character_portrait_map.get_mut(p1_max_arg as i32));
                println!("Player 2 : {} {}", p2_max_arg, p2_max);
                println!("{}",ai_data.character_portrait_map.get_mut(p2_max_arg as i32));

                //NOTE
                //More debugging rendering

                if ai_data.portrait_diagnostics_render{
                    //Player1
                    let _offsetx = 30;
                    let _offsety = 30;
                    for i in (0..98){
                        for j in (0..98){
                            let cursor = ( (97-i) + (97-j)*98 ) * 3;
                            let _r = arr_p1[cursor];
                            let _g = arr_p1[cursor + 1];
                            let _b = arr_p1[cursor + 2];
                            drawRect(&mut GLOBAL_BACKBUFFER, [i as i32 + _offsetx, j as i32 + _offsety, 1, 1],
                                                             [_r, _g, _b, 1.0], true);
                        }
                    }

                    for i in (0..98){
                        for j in (0..98){
                            let cursor = ( (97-i) + (97-j)*98 ) * 3;
                            let _r = arr_p1[cursor];
                            let _g = arr_p1[cursor + 1];
                            let _b = arr_p1[cursor + 2];
                            drawRect(&mut GLOBAL_BACKBUFFER, [i as i32 + _offsetx + 30, j as i32 + _offsety, 1, 1],
                                                             [_r, _g, _b, 1.0], true);
                        }
                    }

                }


            }




        }
        drawBMP(&mut GLOBAL_BACKBUFFER, &screen[0], 330, 100, 1.0, Some(640), Some(360) );
    }
    drawString(&mut GLOBAL_BACKBUFFER, "Something about an ai sailor", 350, 460, [1.0, 1.0, 1.0, 1.0], 40.0);


    let coor_glyph_ai = [20, 400];
    drawString(&mut GLOBAL_BACKBUFFER, "Toggle ai:", 20, 450, [1.0, 1.0, 1.0, 1.0], 24.0);
    drawString(&mut GLOBAL_BACKBUFFER, " [+] glyph classification", coor_glyph_ai[0], coor_glyph_ai[1], [1.0, 1.0, 1.0, 1.0], 20.0);

    //BMP outline
    drawRect(&mut GLOBAL_BACKBUFFER, [330, 100, 640, 360], [0.8, 0.8, 0.8, 1.0], false);

}
    menu(app_data, keyboardinfo, textinfo, mouseinfo, frames, time_instance);
    return 0;
}


fn app_screencapture(app_data: &mut AppData, keyboardinfo: &KeyboardInfo,
    textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize, time_instance: std::time::Duration)->i32{unsafe{


    drawRect(&mut GLOBAL_BACKBUFFER, [20, 25, 30, 30], [1.0, 0.0, (frames%255) as f32 / 255.0, 1.0], true);

    {//We capture the things
        drawString(&mut GLOBAL_BACKBUFFER, "Capturing: ", 300, 450, [1.0, 1.0, 1.0, 1.0], 32.0);
        app_data.capture_exe_textbox.text_color = [0.85, 0.55, 0.65, 1.0];
        app_data.capture_exe_textbox.x = 420;
        app_data.capture_exe_textbox.y = 450;
        app_data.capture_exe_textbox.text_size = 34.0;
        app_data.capture_exe_textbox.max_render_length = 400;

        let x = app_data.capture_exe_textbox.x;
        let y = app_data.capture_exe_textbox.y;
        app_data.capture_exe_update_box= [x + 4, y - 20 + 4, 346, 20];
        if app_data.capture_exe_update_text_color[3] > 0.1{
            app_data.capture_exe_update_text_color[3] *= 0.97;
            app_data.capture_exe_update_box_color[3] *= 0.97;
        } else {
            app_data.capture_exe_update_text_color[3] = 0.0;
            app_data.capture_exe_update_box_color[3] = 0.0;
        }

        let pre_textbox_active = app_data.capture_exe_textbox.active;
        app_data.capture_exe_textbox.update(keyboardinfo, textinfo, mouseinfo);

        let mut update_exe = false;
        if app_data.capture_exe_textbox.active == false && pre_textbox_active == true{

            for (i, it) in keyboardinfo.key.iter().enumerate() {
                if *it == KeyboardEnum::Enter && keyboardinfo.status[i] == ButtonStatus::Down{
                    update_exe = true;
                }
            }
        }
        if update_exe {
            if foundWindow(&app_data.capture_exe_textbox.text_buffer) == false {
                app_data.capture_exe_update_text = "Could not find window".to_string();
                app_data.capture_exe_update_text_color[3] = 1.0;
                app_data.capture_exe_update_box_color = [1.0, 0.2, 0.0, 1.0];
            } else{
                app_data.capture_exe_update_text = "Window Found".to_string();
                app_data.capture_exe_update_text_color[3] = 1.0;
                app_data.capture_exe_update_box_color = [0.1, 1.0, 0.1, 1.0];
            }
        }
        drawRect(&mut GLOBAL_BACKBUFFER, app_data.capture_exe_update_box, app_data.capture_exe_update_box_color, true);
        let _x = app_data.capture_exe_update_box[0];
        let _y = app_data.capture_exe_update_box[1] - 4;
        drawString(&mut GLOBAL_BACKBUFFER, &app_data.capture_exe_update_text, _x, _y,
                    app_data.capture_exe_update_text_color, app_data.capture_exe_update_text_size);
        app_data.capture_exe_textbox.draw(time_instance.subsec_nanos() as f32);
    }

    {//We save the things to this directory
        let x = 50;
        let y = 350;
        drawString(&mut GLOBAL_BACKBUFFER, "Current directory name:", x, y, [1.0, 1.0, 1.0, 1.0], 24.0);
        app_data.root_folder_textbox.text_color = [0.85, 0.55, 0.65, 1.0];
        app_data.root_folder_textbox.x = x + 20;
        app_data.root_folder_textbox.y = y - 20;
        app_data.root_folder_textbox.text_size = 22.0;
        app_data.root_folder_textbox.max_render_length = 325;

        app_data.root_folder_update_box= [x+20 + 4, y - 40 + 4, 346, 20];
        if app_data.root_folder_update_text_color[3] > 0.1{
            app_data.root_folder_update_text_color[3] *= 0.97;
            app_data.root_folder_update_box_color[3] *= 0.97;
        } else {
            app_data.root_folder_update_text_color[3] = 0.0;
            app_data.root_folder_update_box_color[3] = 0.0;
        }


        let pre_textbox_active = app_data.root_folder_textbox.active;
        app_data.root_folder_textbox.update(keyboardinfo, textinfo, mouseinfo);

        if app_data.root_folder_textbox.active == false && pre_textbox_active == true
        {// maybe we want to create a directory
            let mut enter_pressed = false;
            for i in 0..keyboardinfo.key.len(){
                if keyboardinfo.key[i] == KeyboardEnum::Enter &&
                   keyboardinfo.status[i] == ButtonStatus::Down{
                       enter_pressed = true;
                       break;
                }
            }
            let mut good_root_folder_path = true;
            if enter_pressed {
                if app_data.root_folder_textbox.text_buffer.len() == 0 {
                    app_data.root_folder_update_text = "You can not make a folder with empty string".to_string();
                    good_root_folder_path = false;
                }
                else {
                    //TODO
                    //make more robust
                    for it in read_dir("").expect("Could not read directory"){
                        let _dir_in_path = it.expect("Could not access directory in path.").path();
                        let dir_in_path = _dir_in_path.as_path().to_str().expect("Could not convert path to string.");
                        if dir_in_path ==  app_data.root_folder_textbox.text_buffer{
                            app_data.root_folder_update_text= "Directory name collision!".to_string();
                            good_root_folder_path = false;
                            break;
                        }
                    }
                }

                if good_root_folder_path {
                    create_dir(&app_data.root_folder_textbox.text_buffer).expect("Directory was not created.");
                    app_data.root_folder_update_text= "Directory created!".to_string();
                    app_data.root_folder_update_box_color= [0.1, 1.0, 0.1, 1.0];
                } else {
                    app_data.root_folder_update_box_color= [1.0, 0.2, 0.0, 1.0];
                }
                app_data.root_folder_update_text_color[3]=  1.0;
                app_data.root_folder_update_box_color[3]= 1.0;
            }
        }

        drawRect(&mut GLOBAL_BACKBUFFER, app_data.root_folder_update_box, app_data.root_folder_update_box_color, true);
        let _x = app_data.root_folder_update_box[0];
        let _y = app_data.root_folder_update_box[1] - 4;
        drawString(&mut GLOBAL_BACKBUFFER, &app_data.root_folder_update_text, _x, _y,
                    app_data.root_folder_update_text_color, app_data.root_folder_update_text_size);
        app_data.root_folder_textbox.draw(time_instance.subsec_nanos() as f32);
    }


    {//The things are named thusly
        let x = 50;
        let y = 250;
        drawString(&mut GLOBAL_BACKBUFFER, "File name:", x, y, [1.0, 1.0, 1.0, 1.0], 24.0);
        app_data.image_prepend_name_textbox.text_color = [0.85, 0.55, 0.65, 1.0];
        app_data.image_prepend_name_textbox.x = x + 20;
        app_data.image_prepend_name_textbox.y = y - 20;
        app_data.image_prepend_name_textbox.text_size = 22.0;
        app_data.image_prepend_name_textbox.max_render_length = 325;
        app_data.image_prepend_name_textbox.update(keyboardinfo, textinfo, mouseinfo);


        app_data.image_prepend_name_textbox.draw(time_instance.subsec_nanos() as f32);
    }

    {//Number of screenshots taken
        let x = 230;
        let y = 160;
        let x_off = 70;

        app_data.number_of_shots_to_take_textbox.text_color = [0.85, 0.55, 0.65, 1.0];
        app_data.number_of_shots_to_take_textbox.x = x+x_off;
        app_data.number_of_shots_to_take_textbox.y = y;
        app_data.number_of_shots_to_take_textbox.text_size = 24.0;
        app_data.number_of_shots_to_take_textbox.max_render_length = 30;
        app_data.number_of_shots_to_take_textbox.max_char = 2;

        let old_box = app_data.number_of_shots_to_take_textbox.clone();
        app_data.number_of_shots_to_take_textbox.update(keyboardinfo, textinfo, mouseinfo);

        if app_data.number_of_shots_to_take_textbox.text_buffer.len() == 0 {
            app_data.number_of_shots_to_take_textbox = old_box.clone();
            app_data.number_of_shots_to_take_textbox.text_buffer = "0".to_string();
        }
        if app_data.number_of_shots_to_take_textbox.text_buffer.parse::<i32>().is_ok() == false{
            app_data.number_of_shots_to_take_textbox = old_box.clone();
        }

        drawString(&mut GLOBAL_BACKBUFFER, "Capture           frames.", x, y, [1.0, 1.0, 1.0, 1.0], 24.0);
        //Do we need buttons?
        //drawRect(&mut GLOBAL_BACKBUFFER, [x+x_off+9, y+32, 20, 20], [1.0,1.0,1.0,1.0], true);
        //drawRect(&mut GLOBAL_BACKBUFFER, [x+x_off+9, y-20, 20, 20], [1.0,1.0,1.0,1.0], true);

        app_data.number_of_shots_to_take_textbox.draw( time_instance.subsec_nanos() as f32);

    }

    {//Select input for to poll for screen shot
        let mut alpha_gp = 0.1;
        let mut alpha_kb = 0.1;

        app_data.cameratrigger.recently_updated += 1;

        {
            let w  = app_data.gamepad_bmp.info_header.width;
            let h  = app_data.gamepad_bmp.info_header.height;
            if in_rect(mouseinfo.x, mouseinfo.y, [20, 150, w, h]){
                alpha_gp = 1.0;

                if mouseinfo.lbutton == ButtonStatus::Down &&
                   app_data.cameratrigger_is_updating == false{

                    app_data.cameratrigger_is_updating = true;
                    app_data.cameratrigger = CameraTrigger{id: 0x00,
                                                           trigger_type: TriggerType::Gamepad,
                                                            recently_updated: 0};
                }
            }
            if app_data.cameratrigger.id == 0x00 &&
               app_data.cameratrigger_is_updating &&
               app_data.cameratrigger.trigger_type == TriggerType::Gamepad{

                alpha_gp = 1.0;
                let mut temp_xgamepad = xinput::XINPUT_STATE{dwPacketNumber: 0,
                                                         Gamepad: xinput::XINPUT_GAMEPAD{
                                                            wButtons: 0,
                                                            bLeftTrigger: 0,
                                                            bRightTrigger: 0,
                                                            sThumbLX: 0,
                                                            sThumbLY: 0,
                                                            sThumbRX: 0,
                                                            sThumbRY: 0,
                                                        }};
                xinput::XInputGetState(0, &mut temp_xgamepad as *mut xinput::XINPUT_STATE);
                if temp_xgamepad.Gamepad.wButtons != 0x00 {
                    app_data.cameratrigger_is_updating = false;
                    app_data.cameratrigger.id = temp_xgamepad.Gamepad.wButtons as u32;
                    app_data.cameratrigger.recently_updated = 0;
                }
            }
        }
        {
            let w  = app_data.keyboard_bmp.info_header.width;
            let h  = app_data.keyboard_bmp.info_header.height;
            if in_rect(mouseinfo.x, mouseinfo.y, [100, 150, w, h]){
                alpha_kb = 1.0;
                if mouseinfo.lbutton == ButtonStatus::Down{
                    app_data.cameratrigger = CameraTrigger{id: 0x20, trigger_type: TriggerType::Keyboard, recently_updated: 0};
                }
            }
        }
        drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.gamepad_bmp, 20, 150, alpha_gp, None, None);
        drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.keyboard_bmp, 100, 150, alpha_kb, None, None);
        drawString(&mut GLOBAL_BACKBUFFER, &format!("CameraTrigger:      id= {}   trigger_type= {:?}", app_data.cameratrigger.id, app_data.cameratrigger.trigger_type),
                    25, 120, [0.7, 0.7, 0.7, 1.0], 24.0);
        drawString(&mut GLOBAL_BACKBUFFER, &format!("Frames since update: {} ", app_data.cameratrigger.recently_updated),
                    25, 96, [0.7, 0.7, 0.7, 1.0], 24.0);
    }

    //NOTE
    //We use the OS direct call because we wnat to be able to capture image with out being on this
    //apps window
    {//TODO
    //Resturcture the following thing are getting a lil hairy

        let mut screenshot_trigger_activated = false;
        if app_data.cameratrigger.recently_updated > 120{
            match app_data.cameratrigger.trigger_type{
                TriggerType::Keyboard => {
                    if user32::GetAsyncKeyState(app_data.cameratrigger.id as i32) != 0{
                        screenshot_trigger_activated = true;
                    }
                },
                TriggerType::Gamepad => {
                    let mut temp_xgamepad = xinput::XINPUT_STATE{dwPacketNumber: 0,
                                                             Gamepad: xinput::XINPUT_GAMEPAD{
                                                                wButtons: 0,
                                                                bLeftTrigger: 0,
                                                                bRightTrigger: 0,
                                                                sThumbLX: 0,
                                                                sThumbLY: 0,
                                                                sThumbRX: 0,
                                                                sThumbRY: 0,
                                                            }};
                    xinput::XInputGetState(0, &mut temp_xgamepad as *mut xinput::XINPUT_STATE);
                    if temp_xgamepad.Gamepad.wButtons as u32 == app_data.cameratrigger.id &&
                       app_data.cameratrigger.id != 0x00{
                        screenshot_trigger_activated = true;
                    }
                },
            }
        }

        if app_data.capture_exe_textbox.active == false &&
           app_data.root_folder_textbox.active == false &&
           app_data.image_prepend_name_textbox.active == false &&
           app_data.number_of_shots_to_take_textbox.active == false &&
           screenshot_trigger_activated {
               //TODO
               //We need to make sure we do not crash if things are not found
               if !app_data.handle_dc.is_some(){
                   if app_data.capture_exe_textbox.text_buffer.len() > 0 &&
                      foundWindow(&app_data.capture_exe_textbox.text_buffer){
                       app_data.handle_dc = Some(load_handle_dc(&app_data.capture_exe_textbox.text_buffer));
                   }
               }
               //TODO
               //change screenshot number to a variable that changes or resets when the name of the
               //screen shot is changed

               let n_shots_to_capture = app_data.number_of_shots_to_take_textbox.text_buffer.parse::<i32>().expect("Number of screenshots to be taken is not a i32");
               let file_name = app_data.image_prepend_name_textbox.text_buffer.clone() + &format!("_{}", app_data.image_counter);
               let mut arr = screen_shot(app_data.handle_dc.as_ref().expect("App data dc could not be taken as a reference."), n_shots_to_capture,
                                         &file_name,
                                         &app_data.root_folder_textbox.text_buffer, true);
               app_data.screenshot_buffer.append(&mut arr);
               app_data.currently_rendering_index = app_data.screenshot_buffer.len() - 1;
               app_data.image_counter += 1;
        }



        drawString(&mut GLOBAL_BACKBUFFER, &format!("Number of frames captured: {}", app_data.screenshot_buffer.len()),
                    420, 400, [1.0, 1.0, 1.0, 1.0], 26.0);
        if app_data.screenshot_buffer.len() > 0 {
            if app_data.currently_rendering_index == app_data.old_rendering_index{
                match app_data.currently_rendering_screenshot{
                    Some(ref temp_bmp) =>{
                        drawBMP(&mut GLOBAL_BACKBUFFER, temp_bmp, 420, 100, 1.0, None, None);
                        app_data.old_rendering_index = app_data.currently_rendering_index;
                    },
                    None => {
                        let index = app_data.currently_rendering_index;
                        app_data.old_rendering_index = app_data.currently_rendering_index;
                        let temp_bmp = resizeBMP(&app_data.screenshot_buffer[index], 532, 300);
                        drawBMP(&mut GLOBAL_BACKBUFFER, &temp_bmp, 420, 100, 1.0, None, None);
                        app_data.currently_rendering_screenshot = Some( temp_bmp);
                    }
                }
            } else {
                let index = app_data.currently_rendering_index;
                app_data.old_rendering_index = app_data.currently_rendering_index;
                let temp_bmp = resizeBMP(&app_data.screenshot_buffer[index], 532, 300);
                drawBMP(&mut GLOBAL_BACKBUFFER, &temp_bmp, 420, 100, 1.0, None, None);
                app_data.currently_rendering_screenshot = Some( temp_bmp);
            }
            {
                let r_x = 900;
                let r_y = 200;
                let r_w = 100;
                let r_h = 100;
                if app_data.arrow_right_alpha > 0.0 {
                    app_data.arrow_right_alpha -= 0.01;
                    if app_data.arrow_right_alpha < 0.0 { app_data.arrow_right_alpha = 0.0;}
                }
                if in_rect(mouseinfo.x, mouseinfo.y, [r_x, r_y, r_w, r_h]){ //Right arrow
                    app_data.arrow_right_alpha = 0.5;
                    if mouseinfo.lbutton == ButtonStatus::Down && mouseinfo.old_lbutton != mouseinfo.lbutton{
                        app_data.currently_rendering_index += 1;
                        if app_data.currently_rendering_index >= app_data.screenshot_buffer.len(){
                            app_data.currently_rendering_index = 0;
                        }
                    }
                }
                drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.arrow_right_bmp, r_x, r_y, app_data.arrow_right_alpha, Some(r_w), Some(r_h));
            }
            {
                let l_x = 400;
                let l_y = 200;
                let l_w = 100;
                let l_h = 100;
                if app_data.arrow_left_alpha > 0.0 {
                    app_data.arrow_left_alpha -= 0.01;
                    if app_data.arrow_left_alpha < 0.0{ app_data.arrow_left_alpha = 0.0;}
                }
                if in_rect(mouseinfo.x, mouseinfo.y, [l_x, l_y, l_w, l_h]){ //Right arrow
                    app_data.arrow_left_alpha = 0.5;
                    if mouseinfo.lbutton == ButtonStatus::Down && mouseinfo.old_lbutton != mouseinfo.lbutton{
                        if app_data.currently_rendering_index == 0{
                            app_data.currently_rendering_index = app_data.screenshot_buffer.len() - 1;
                        } else {
                            app_data.currently_rendering_index -= 1;
                        }
                    }
                }
                drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.arrow_left_bmp, l_x, l_y, app_data.arrow_left_alpha, Some(l_w), Some(l_h));
            }
        }
    }

    //this is where we are going to place our last screen shot
    drawRect(&mut GLOBAL_BACKBUFFER, [420, 100, 532, 300], [0.4, 0.5, 1.0, 1.0], false);


    if in_rect(mouseinfo.x, mouseinfo.y, [40, 50, 10, 10]){
        drawRect(&mut GLOBAL_BACKBUFFER, [40, 50, 10, 10], [0.4, 0.5, (frames%255) as f32 / 255.0, 1.0], false);
    }
    else{
        drawRect(&mut GLOBAL_BACKBUFFER, [40, 50, 10, 10], [1.0, 0.5, (frames%255) as f32 / 255.0, 1.0], false);
    }
    drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.temp_bmp, 20, 20, 1.0, None, None);

    menu(app_data, keyboardinfo, textinfo, mouseinfo, frames, time_instance);
    return 0;
}}

fn in_rect(x: i32, y: i32, rect: [i32;4])->bool{
    let mut rt = true;
    if x < rect[0]{
        rt = false;
    }
    if y < rect[1]{
        rt = false;
    }
    if x > rect[0] + rect[2]{
        rt = false;
    }
    if y > rect[1] + rect[3]{
        rt = false;
    }
    return rt;
}
fn foundWindow(name: &str)->bool{unsafe{
    use std::iter::once;
    use user32::FindWindowW;

    let mut rt = true;
    let windows_string: Vec<u16> = OsStr::new(name).encode_wide().chain(once(0)).collect();
    let window_hwnd = FindWindowW(null_mut(), windows_string.as_ptr());

    if window_hwnd == null_mut() {
        rt = false;
    }
    gdi32::DeleteDC(window_hwnd as HDC);

    return rt;
}}


fn print_message(msg: &str) -> Result<i32, Error> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::MB_OK;
    use user32::MessageBoxW;
    let wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    let ret = unsafe {
        MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), MB_OK)
    };
    if ret == 0 { Err(Error::last_os_error()) }
    else { Ok(ret) }
}








//END
