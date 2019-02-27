extern crate winapi;
extern crate stb_tt_sys;



use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::iter::once;
use std::io;
use std::io::Error;
use std::io::prelude::*;
use std::mem;
use std::io::prelude::*;
use std::fs::{File, create_dir, read_dir};
use std::{ thread, time };
use std::ptr::{null, null_mut};
use winapi::shared::windef::{HWND, RECT, HDC, HBITMAP, HWND__, HDC__};
use winapi::um::wingdi::{BITMAP, BITMAPINFO, BITMAPINFOHEADER, SRCCOPY, RGBQUAD};
use winapi::um::wingdi as gdi32;
use winapi::um::winuser as user32;
use winapi::um::libloaderapi as kernel32;
use winapi::um::xinput;
use stb_tt_sys::*;


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

struct WindowHandleDC{
    window_handle : *mut HWND__,
    window_dc     : *mut HDC__,
}

fn load_handle_dc(window_name: &str, )->WindowHandleDC{ unsafe{
    use std::iter::once;
    use user32::{FindWindowW, GetWindowDC};

    let windows_string: Vec<u16> = OsStr::new(window_name).encode_wide().chain(once(0)).collect();

    let handle = FindWindowW(null_mut(), windows_string.as_ptr());
    let handle_dc = WindowHandleDC{ window_handle: handle,
                    window_dc: GetWindowDC(handle)};

    return handle_dc;
}}

fn new_rgbquad()->RGBQUAD{
     RGBQUAD{
        rgbBlue: 0,
        rgbGreen: 0,
        rgbRed: 0,
        rgbReserved: 0,
     }
}


fn screen_shot(handle_dc: &WindowHandleDC, number_of_shots: i32, file_prepend: &str, directory_prepend: &str)->Vec<TGBitmap>{unsafe{
    use gdi32::{CreateCompatibleBitmap, SelectObject, BitBlt, CreateCompatibleDC, GetObjectA, GetObjectW, GetDIBits};

    let mut rt = Vec::new();

    let mut rect: RECT = RECT{ left: 0, top: 0, right: 0, bottom: 0};
    if user32::GetWindowRect(handle_dc.window_handle, &mut rect) != 0{
    } else {
        println!("Coud not get window rect");
        return rt;
    }

    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    //DO WE NNED TO FREE THIS?
    let mut bitmap_handle = CreateCompatibleBitmap( handle_dc.window_dc, w, h);

    if bitmap_handle == null_mut(){
        println!("bitmap was bad.");
    }
    //DO WE NNED TO FREE THIS?
    let mut compat_dc = CreateCompatibleDC(handle_dc.window_dc);
    let mut _wi = 0;
    while true {
        let esc = user32::GetAsyncKeyState(0x1B);
        if esc != 0 {
            println!("escape");
            break;
        }

        if _wi == number_of_shots{
            break;
        }
        _wi = _wi + 1;
        let oldBitmap = SelectObject(compat_dc, bitmap_handle as winapi::shared::windef::HGDIOBJ);
        let bRet      = BitBlt(compat_dc as HDC, 0, 0, w, h,
                                    handle_dc.window_dc as HDC, 0, 0, SRCCOPY);

        //https://stackoverflow.com/questions/3291167/how-can-i-take-a-screenshot-in-a-windows-application
        //https://msdn.microsoft.com/en-us/library/windows/desktop/dd183402(v=vs.85).aspx
        //https://stackoverflow.com/questions/31302185/rust-ffi-casting-to-void-pointer
        let mut pixels = vec![0u8; (4*w*h) as usize]; //TODO might want to change this to an array of u8s
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
            bmiColors: [new_rgbquad()], //[]//DIB_RGB_COLORS,
        };

        GetDIBits(handle_dc.window_dc, bitmap_handle, 0, bitmap.bmHeight as u32, &mut pixels[0] as *mut u8 as *mut winapi::ctypes::c_void, &mut bitmap_info as *mut BITMAPINFO, 0);
        SelectObject(compat_dc, oldBitmap);



        let mut header =  TGBitmapFileHeader{   type_: 0x4d42, //BM
                                                size_:(mem::size_of::<TGBitmapFileHeader>() + mem::size_of::<TGBitmapHeaderInfo>() + 4 * pixels.len()) as u32,
                                                reserved_1: 0,
                                                reserved_2: 0,
                                                off_bits: (mem::size_of::<TGBitmapFileHeader>() + mem::size_of::<TGBitmapHeaderInfo>()) as u32};

        //Redunant please fix
        let mut info = TGBitmapHeaderInfo{
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

        rt.push(TGBitmap{file_header: header, info_header: info, rgba: pixels});

    }
    for it in rt.iter(){
        let mut filebuffer = File::create(format!("{}/{}_{:}.bmp",directory_prepend, file_prepend, _wi)).unwrap();
        //filebuffer.write( &transmute(&header) ).unwrap();
        {
            filebuffer.write( &transmute(&it.file_header.type_) ).unwrap();
            filebuffer.write( &transmute(&it.file_header.size_) ).unwrap();
            filebuffer.write( &transmute(&it.file_header.reserved_1) ).unwrap();
            filebuffer.write( &transmute(&it.file_header.reserved_2) ).unwrap();
            filebuffer.write( &transmute(&it.file_header.off_bits) ).unwrap();
        }
        {
            //filebuffer.write( &transmute(&info) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.header_size) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.width) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.height) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.planes) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.bit_per_pixel) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.compression) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.image_size) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.x_px_per_meter) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.y_px_per_meter) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.colors_used) ).unwrap();
            filebuffer.write( &transmute(&it.info_header.colors_important) ).unwrap();
        }
        filebuffer.write( &it.rgba ).unwrap();
    }
    gdi32::DeleteDC(compat_dc as HDC);
    gdi32::DeleteDC(bitmap_handle as HDC);
    return rt;
}}

struct BitmapFileHeader{
    type_:              u16,
    size_:              u32,
    reserved_1:         u16,
    reserved_2:         u16,
    off_bits:           u32,
}

fn transmute<T>(t:&T)->Vec<u8>{unsafe{
    let ptr = t as *const T as *const u8;
    let mut v = vec![];
    for i in 0..mem::size_of::<T>(){
        v.push(*ptr.offset(i as isize));
    }
    v
}}
#[derive(Debug)]
#[derive(Default)]
struct TGBitmapHeaderInfo{
    header_size:        u32,
    width:              i32,
    height:             i32,
    planes:             u16,
    bit_per_pixel:      u16,
    compression:        u32,
    image_size:         u32,
    x_px_per_meter:     i32,
    y_px_per_meter:     i32,
    colors_used:        u32,
    colors_important:   u32,
}

//struct palette
//array of pixels

#[repr(packed)]
struct TGBitmapFileHeader{
    type_:              u16,
    size_:              u32,
    reserved_1:         u16,
    reserved_2:         u16,
    off_bits:           u32,
}


struct TGBitmap{
    file_header:        TGBitmapFileHeader,
    info_header:        TGBitmapHeaderInfo,
    rgba:               Vec<u8>,
}

impl TGBitmap{
    fn new(w: i32, h: i32)->TGBitmap{
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
            rgba:               vec![0;4 * (w*h) as usize],
        }

    }
}


struct WindowsCanvas{
    info : BITMAPINFO,
    w: i32,
    h: i32,
    buffer: *mut std::ffi::c_void
}
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
                rgbReserved: 0, }],//[]//DIB_RGB_COLORS,
    },
    w : 0,
    h : 0,
    buffer : null_mut(),
};

fn renderToBuffer( canvas: &mut WindowsCanvas){unsafe{
    let mut buffer = canvas.buffer as *mut u32;
    let w = canvas.w;
    let h = canvas.h;

    for i in 0..(w*h) as isize {
        *buffer.offset(i) = 0x00000000 + (100 << 16) +  (50 << 8)  + 50;
    }
}}

static mut GLOBAL_FONTINFO : stbtt_fontinfo = new_stbtt_fontinfo();

fn resizeBMP(source_bmp: &TGBitmap, w: i32, h: i32)->TGBitmap{unsafe{
    let mut bmp = TGBitmap::new(w, h);
    {
        //we need to determine a way to bin our input BMP
        if source_bmp.info_header.width < w{
            println!("Trash", );
        }
        if source_bmp.info_header.height < h{
            println!("Trash");
        }
        let scale_w = w as f32 / source_bmp.info_header.width as f32;
        let scale_h = h as f32 / source_bmp.info_header.height as f32;

        let mut source_buffer = source_bmp.rgba.as_ptr();
        let mut dst_buffer = bmp.rgba.as_mut_ptr() as *mut u32;

        let bytes_per_pix = (source_bmp.info_header.bit_per_pixel / 8) as isize;

        for i in 0..source_bmp.info_header.width{
            for j in 0..source_bmp.info_header.height{
                let mut _i = (i as f32 * scale_w).round() as i32;
                let mut _j = (j as f32 * scale_h).round() as i32;

                if _i >= w { _i = w-1; }
                if _j >= h { _j = h-1; }


                let src_rgb = source_buffer.offset(  bytes_per_pix * (i + source_bmp.info_header.width * j) as isize);
                let src_r =  *(src_rgb as *const u8).offset(2);
                let src_g =  *(src_rgb as *const u8).offset(1);
                let src_b =  *(src_rgb as *const u8).offset(0);

                let r = (src_r as f32 * scale_w * scale_h) as u32;
                let g = (src_g as f32 * scale_w * scale_h) as u32;
                let b = (src_b as f32 * scale_w * scale_h) as u32;

                *dst_buffer.offset( (_i + w * _j) as isize ) += 0x00000000 + (r << 16) + (g << 8) + b;
            }
        }
    }
    return bmp;
}}



fn drawBMP( canvas: &mut WindowsCanvas, source_bmp: &TGBitmap, x: i32, y: i32, alpha: f32,
            _w: Option<i32>, _h: Option<i32>){unsafe{
    //TODO
    //This is super slow need to make this faster
    let mut w = 0;
    let mut h = 0;
    match _w {
        Some(int) => w = int,
        None => w = source_bmp.info_header.width,
    }
    match _h {
        Some(int) => h = int,
        None => h = source_bmp.info_header.height,
    }

    //TODO
    //We should remove this are make it not happend unless ther is a real scaling being done
    let mut bmp = TGBitmap::new(w, h);
    {
        //we need to determine a way to bin our input BMP
        if source_bmp.info_header.width < w{
            println!("Trash", );
        }
        if source_bmp.info_header.height < h{
            println!("Trash");
        }
        let scale_w = w as f32 / source_bmp.info_header.width as f32;
        let scale_h = h as f32 / source_bmp.info_header.height as f32;

        let mut source_buffer = source_bmp.rgba.as_ptr();
        let mut dst_buffer = bmp.rgba.as_mut_ptr() as *mut u32;

        let bytes_per_pix = (source_bmp.info_header.bit_per_pixel / 8) as isize;

        for i in 0..source_bmp.info_header.width{
            for j in 0..source_bmp.info_header.height{
                let mut _i = (i as f32 * scale_w).round() as i32;
                let mut _j = (j as f32 * scale_h).round() as i32;

                if _i >= w { _i = w-1; }
                if _j >= h { _j = h-1; }

                //TODO
                //Remove me
                if j == 600{
   //                 println!("{} {} {} {}", scale_w,  scale_h, h, source_bmp.info_header.height);
                }

                let src_rgb = source_buffer.offset(  bytes_per_pix * (i + source_bmp.info_header.width * j) as isize);
                let src_r =  *(src_rgb as *const u8).offset(2);
                let src_g =  *(src_rgb as *const u8).offset(1);
                let src_b =  *(src_rgb as *const u8).offset(0);

                let r = (src_r as f32 * scale_w * scale_h) as u32;
                let g = (src_g as f32 * scale_w * scale_h) as u32;
                let b = (src_b as f32 * scale_w * scale_h) as u32;

                *dst_buffer.offset( (_i + w * _j) as isize ) += 0x00000000 + (r << 16) + (g << 8) + b;
                //TODO
                //Remove me
                //println!( "{:x} {:x}", *src_rgb, *dst_buffer.offset( (_i + w * _j) as isize ));

            }
        }
    }

    {   //render bmp_buffer to main_buffer

        let buffer = canvas.buffer as *mut u32;
        let gwidth = canvas.w as i32;
        let gheight = canvas.h as i32;
        let offset = (x + y * gwidth) as i32;

        let color = bmp.rgba.as_ptr();
        for i in (0..bmp.info_header.height).rev(){
            for j in 0..bmp.info_header.width{


                if (j + i*gwidth + offset) < 0 {continue;}
                if (j + i*gwidth + offset) > gwidth * gheight {continue;}

                if j + x > gwidth {continue;}
                if i + y > gheight {continue;}


                let r = (*color.offset(( 4*(j + i * bmp.info_header.width) + 2) as isize) as f32 * alpha ) as u32;
                let g = (*color.offset(( 4*(j + i * bmp.info_header.width) + 1) as isize) as f32 * alpha ) as u32;
                let b = (*color.offset(( 4*(j + i * bmp.info_header.width) + 0) as isize) as f32 * alpha ) as u32;

                let dst_rgb = buffer.offset( (j + i*gwidth + offset) as isize);
                let _r = (*(dst_rgb as *const u8).offset(2) as f32 * (1.0 - alpha )) as u32;
                let _g = (*(dst_rgb as *const u8).offset(1) as f32 * (1.0 - alpha )) as u32;
                let _b = (*(dst_rgb as *const u8).offset(0) as f32 * (1.0 - alpha )) as u32;

                *buffer.offset( (j + i*gwidth + offset) as isize) = 0x00000000 + (r+_r << 16) + (g+_g << 8) + b+_b;
                //TODO
                //Remove me
                if i == 100 {
                    //println!("{} {} {} \t {} {} {}", r, g, b, _r, _g, _b);
                    //println!( "{:x} ",  *buffer.offset( (j +bmp.info_header.width * j + offset) as isize ));
                }
            }
        }
    }
}}


fn drawChar( canvas: &mut WindowsCanvas, character: char, x: i32, y: i32,
             color: [f32; 4], size: f32 )->i32{unsafe{
    //TODO set up alpha
    //Check that globalfontinfo has been set
    if GLOBAL_FONTINFO.data == null_mut() {
        println!("Global font has not been set.");
        return -1;
    }
    //let mut now = time::Instant::now();

    //construct a char buffer
    let mut char_buffer = Vec::new();
    let mut cwidth = 0;
    let mut cheight = 0;
    let mut scale = 0.0;
    {//NOTE
     //this accounts for about 10% of character rendering time.
     //If we want an easy speed up we can save the results to a global buffer  map
     // can only add to it when there is a new character being renedered
     // how ever if we build in release mode it doesn't really matter
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

    if character as u8 > 0x20{   //render char_buffer to main_buffer
        let buffer = canvas.buffer as *mut u32;
        let gwidth = canvas.w as usize;
        let gheight = canvas.h as usize;
        let offset = (x as usize + y as usize * gwidth) as usize;
        for i in 0..cheight{
            for j in 0..cwidth{

                if (j + i*gwidth + offset) < 0 {continue;}
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

fn getAdvance(character: char, size: f32)->i32{unsafe{
    if GLOBAL_FONTINFO.data == null_mut() {
        println!("Global font has not been set.");
        return -1;
    }
    let mut adv = 0;
    let mut scale = stbtt_ScaleForPixelHeight(&GLOBAL_FONTINFO as *const stbtt_fontinfo, size);
    stbtt_GetCodepointHMetrics(&GLOBAL_FONTINFO as *const stbtt_fontinfo, character as i32, &mut adv as *mut i32, null_mut());
    return (adv as f32 * scale) as i32;
}}

fn drawString( canvas: &mut WindowsCanvas, string: &str, x: i32, y: i32,
             color: [f32; 4], size: f32 ){
    let mut offset = 0;
    for it in string.chars(){
        offset += drawChar(canvas, it, x + offset, y, color, size);
    }
}
fn drawRect( canvas: &mut WindowsCanvas, rect: [i32; 4], color: [f32; 4], filled: bool ){unsafe{
    //TODO
    //handle alpha  parameter
    let mut buffer = canvas.buffer as *mut u32;

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

    if x > 0 && y > 0 && x < c_w && y < c_h{
        for _i in x..x+w{
            let i = _i as isize;
            for _j in y..y+h{
                let j = _j as isize;
                if i > c_w || j > c_h{
                    continue;
                }
                let dst_rgb = buffer.offset( (i + c_w*j) as isize);
                let _r = (*(dst_rgb as *const u8).offset(2) as f32 * (1.0 - a)) as u32;
                let _g = (*(dst_rgb as *const u8).offset(1) as f32 * (1.0 - a)) as u32;
                let _b = (*(dst_rgb as *const u8).offset(0) as f32 * (1.0 - a)) as u32;

                if filled == false{
                    //TODO
                    //This is a little jank. the widths of the line are not even. need to fix at some point.
                    if (w/2 -  (_i - x) ).abs() < w/2 - 1&&
                       (h/2 -  (_j - y) ).abs() < h/2 - 1{ continue; }
                     *buffer.offset(i + c_w*j) = 0x00000000 + (r+_r << 16) +  (g+_g << 8)  + b+_b;

                } else {
                    *buffer.offset(i + c_w*j) = 0x00000000 + (r+_r << 16) +  (g+_g << 8)  + b+_b;
                }
            }
        }
    }
}}


fn resize_drawsection( canvas: &mut WindowsCanvas, w: i32, h: i32){unsafe{
    use gdi32::{CreateDIBSection, DeleteObject, CreateCompatibleDC};
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
    renderToBuffer( &mut GLOBAL_BACKBUFFER);
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
    use gdi32::PatBlt;


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
             //will soon becomre my DrawRect function
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
    use user32::{RegisterClassW, CreateWindowExW, TranslateMessage, DispatchMessageW, GetMessageW, PeekMessageW, LoadCursorW};
    use winapi::um::winuser::{ WNDCLASSW, CW_USEDEFAULT, WS_OVERLAPPEDWINDOW, WS_VISIBLE, MSG, IDC_ARROW};
    use winapi::shared::windef::POINT;

    let instance = kernel32::GetModuleHandleW(null());
    let commandline = winapi::um::processenv::GetCommandLineA();

    let mut mouseinfo = MouseInfo::new();
    let mut app_data = AppData::new();
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
        let mut window_handle = CreateWindowExW(
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
            let mut nframes = 0;
            'a : loop {
                let mut message = MSG{ hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time: 0, pt: POINT{x:0, y:0} };
                let mut textinfo = TextInfo{character: '\0', timing:0};
                let mut keyboardinfo = KeyboardInfo{key: Vec::new(), status:Vec::new()};

                //NOTE
                //I'm not sure this should be here perhaps some where else...
                mouseinfo.old_lbutton = mouseinfo.lbutton.clone();
                while PeekMessageW(&mut message as *mut MSG, null_mut(), 0, 0, 0x0001) > 0{
                    {//NOTE: Handle mouse events
                        //Convert to the correct coordinates
                        mouseinfo.x = message.pt.x - GLOBAL_WINDOWINFO.x - 10;
                        mouseinfo.y = GLOBAL_WINDOWINFO.h - ( message.pt.y - GLOBAL_WINDOWINFO.y) - 10;

                        if message.message == winapi::um::winuser::WM_LBUTTONDOWN{ mouseinfo.lbutton = ButtonStatus::Down; }
                        else if message.message == winapi::um::winuser::WM_LBUTTONUP{ mouseinfo.lbutton = ButtonStatus::Up; }
                        else { mouseinfo.lbutton = ButtonStatus::Default; }
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
                renderToBuffer(&mut GLOBAL_BACKBUFFER);

                if app_main(&mut app_data, &keyboardinfo, &textinfo, &mouseinfo, nframes) != 0 { break 'a; }

                //TODO
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

static mut GLOBAL_WINDOWINFO : WindowInfo = WindowInfo{ x: 0, y: 0, w: 0, h: 0};

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
    let mut f = File::open("assets/NotoSans-Regular.ttf").unwrap();
    let mut font_buffer = Vec::new();
    f.read_to_end(&mut font_buffer).unwrap();
    unsafe{
        if InitFont(&mut GLOBAL_FONTINFO as *mut stbtt_fontinfo, font_buffer.as_ptr(), 0) == 0{
            panic!("font was not able to load.");
        }
    }
    unsafe { xinput::XInputEnable(1); }
    make_window( );
}

//TODO
fn setprivilege(){

}

#[derive(PartialEq, Clone)]
enum ButtonStatus{
    Up,
    Down,
    Default
}
struct MouseInfo{
    x: i32,
    y: i32,
    lbutton: ButtonStatus,
    old_lbutton: ButtonStatus,
}
impl MouseInfo{
    pub fn new()->MouseInfo{
        MouseInfo{
            x: 0,
            y: 0,
            lbutton: ButtonStatus::Default,
            old_lbutton: ButtonStatus::Default
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
        if self.active == false {
            if in_rect(mouseinfo.x, mouseinfo.y,
               [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32]) &&
               mouseinfo.lbutton == ButtonStatus::Down{
                self.active = true;

                {//Look for where to place cursor
                    let mut position = 0;
                    for (i, it) in self.text_buffer.chars().enumerate() {
                        //IF mouse is between old position and new position then we place cursor
                        //behind the current character
                        let adv = getAdvance(it, self.text_size);
                        if mouseinfo.x >= position + self.x+4 && mouseinfo.x < position + adv + self.x + 4 {
                            self.text_cursor = i;
                            break;
                        }
                        position += adv;

                    }

                }
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
                    let mut position = 0;
                    for (i, it) in self.text_buffer.chars().enumerate() {
                        //IF mouse is between old position and new position then we place cursor
                        //behind the current character
                        let adv = getAdvance(it, self.text_size);
                        if mouseinfo.x >= position + self.x+4 && mouseinfo.x < position + adv + self.x + 4 {
                            self.text_cursor = i;
                            break;
                        }
                        position += adv;

                    }
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
            //TODO
            // maybe we should use match instead of if else statements
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
            if self.text_cursor < 0 {
                self.text_cursor = 0;
            }
            if self.text_cursor as usize > self.text_buffer.len() {
                self.text_cursor = self.text_buffer.len();
            }
        }

    }
    pub fn draw(&self){unsafe{
        //TODO
        //Handle background rect thingoo properly
        drawRect(&mut GLOBAL_BACKBUFFER,
             [self.x+4, self.y + 4, self.max_render_length , self.text_size as i32],
             self.bg_color, true);
        drawString(&mut GLOBAL_BACKBUFFER, &self.text_buffer, self.x, self.y, self.text_color, self.text_size);


        if self.active {
            let mut adv = 0;
            for (i, it) in self.text_buffer.chars().enumerate(){

                if i == self.text_cursor as usize {
                    drawRect(&mut GLOBAL_BACKBUFFER, [self.x+adv+4, self.y+4, 2, self.text_size as i32],
                         self.cursor_color, true);
                    break;
                }
                adv += getAdvance(it, self.text_size);
            }
            if self.text_buffer.len() == 0 || self.text_cursor == self.text_buffer.len(){
                drawRect(&mut GLOBAL_BACKBUFFER, [self.x+adv+4, self.y+4, 2, self.text_size as i32],
                     self.cursor_color, true);
            }
        }
    }}
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

struct AppData{
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

    handle_dc: Option<WindowHandleDC>,
}
impl AppData{
    pub fn new()->AppData{
        let mut capture_exe_textbox = TextBox::new();
        capture_exe_textbox.text_buffer = "Guilty Gear Xrd -REVELATOR-".to_string();

        let mut root_folder_textbox = TextBox::new();
        root_folder_textbox.text_buffer = "temp".to_string();

        let mut image_prepend_name_textbox = TextBox::new();
        image_prepend_name_textbox.text_buffer = "text".to_string();

        let mut number_of_shots_to_take_textbox = TextBox::new();
        number_of_shots_to_take_textbox.text_buffer = "1".to_string();
        AppData{
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

            number_of_shots_to_take_textbox: number_of_shots_to_take_textbox,

            screenshot_buffer: Vec::new(),
            currently_rendering_screenshot: None,
            old_rendering_index: 0,
            currently_rendering_index: 0,

            arrow_right_bmp: loadBMP("arrow_right.bmp"),
            arrow_left_bmp: loadBMP("arrow_left.bmp"),
            arrow_right_alpha: 0.0,
            arrow_left_alpha: 0.0,

            gamepad_bmp : loadBMP("gamepad.bmp"),
            keyboard_bmp: loadBMP("keyboard.bmp"),
            cameratrigger: CameraTrigger{id: 0x20,
                                        trigger_type: TriggerType::Keyboard,
                                        recently_updated: 0},
            cameratrigger_is_updating: false,

            temp_bmp: loadBMP("Untitled.bmp"),

            handle_dc: None,
        }
    }
}

fn app_main(app_data: &mut AppData, keyboardinfo: &KeyboardInfo, textinfo: &TextInfo, mouseinfo: &MouseInfo, frames: usize)->i32{unsafe{
//TODO
// savely wrap globalbackbuffer so thata we do not need an unsafe block
    let i = frames%255; //CLEAN THIS UP THIS IS SOOOOOOOO BAD
    drawRect(&mut GLOBAL_BACKBUFFER, [20, 25, 30, 30], [1.0, 0.0, i as f32 / 255.0, 1.0], true);

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
        app_data.capture_exe_textbox.draw();
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
                    for it in read_dir("").unwrap(){
                        let _dir_in_path = it.unwrap().path();
                        let dir_in_path = _dir_in_path.as_path().to_str().unwrap();
                        if dir_in_path ==  app_data.root_folder_textbox.text_buffer{
                            app_data.root_folder_update_text= "Directory name collision!".to_string();
                            good_root_folder_path = false;
                            break;
                        }
                    }
                }

                if good_root_folder_path {
                    create_dir(&app_data.root_folder_textbox.text_buffer);
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
        app_data.root_folder_textbox.draw();
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


        app_data.image_prepend_name_textbox.draw();
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

        app_data.number_of_shots_to_take_textbox.draw();

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
        drawString(&mut GLOBAL_BACKBUFFER, &format!("Frames since updata: {} ", app_data.cameratrigger.recently_updated),
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
               let mut arr = screen_shot(app_data.handle_dc.as_ref().unwrap(), 1,
                                         &app_data.image_prepend_name_textbox.text_buffer,
                                         &app_data.root_folder_textbox.text_buffer);
               app_data.screenshot_buffer.append(&mut arr);
               app_data.currently_rendering_index = app_data.screenshot_buffer.len() - 1;
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
                        let temp_bmp = resizeBMP(&app_data.screenshot_buffer[index], 400, 300);
                        drawBMP(&mut GLOBAL_BACKBUFFER, &temp_bmp, 420, 100, 1.0, None, None);
                        app_data.currently_rendering_screenshot = Some( temp_bmp);
                    }
                }
            } else {
                let index = app_data.currently_rendering_index;
                app_data.old_rendering_index = app_data.currently_rendering_index;
                let temp_bmp = resizeBMP(&app_data.screenshot_buffer[index], 400, 300);
                drawBMP(&mut GLOBAL_BACKBUFFER, &temp_bmp, 420, 100, 1.0, None, None);
                app_data.currently_rendering_screenshot = Some( temp_bmp);
            }
            {
                let r_x = 750;
                let r_y = 200;
                let r_w = 100;
                let r_h = 100;
                if app_data.arrow_right_alpha > 0.0 {
                    app_data.arrow_right_alpha -= 0.01;
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
    drawRect(&mut GLOBAL_BACKBUFFER, [420, 100, 400, 300], [0.4, 0.5, 1.0, 1.0], false);


    if in_rect(mouseinfo.x, mouseinfo.y, [40, 50, 10, 10]){
        drawRect(&mut GLOBAL_BACKBUFFER, [40, 50, 10, 10], [0.4, 0.5, i as f32 / 255.0, 1.0], false);
    }
    else{
        drawRect(&mut GLOBAL_BACKBUFFER, [40, 50, 10, 10], [1.0, 0.5, i as f32 / 255.0, 1.0], false);
    }
    drawBMP(&mut GLOBAL_BACKBUFFER, &app_data.temp_bmp, 20, 20, 1.0, None, None);
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
    use user32::{FindWindowW, GetWindowDC};

    let mut rt = true;
    let windows_string: Vec<u16> = OsStr::new(name).encode_wide().chain(once(0)).collect();
    let window_hwnd = FindWindowW(null_mut(), windows_string.as_ptr());

    if window_hwnd == null_mut() {
        rt = false;
    }
    gdi32::DeleteDC(window_hwnd as HDC);

    return rt;
}}

fn loadBMP(filename: &str)->TGBitmap{unsafe{
    let mut rt = TGBitmap::new(0,0);
    let mut f = File::open(filename).unwrap();
    let mut img_buffer = Vec::new();
    f.read_to_end(&mut img_buffer);

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
