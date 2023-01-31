use libflac_sys::*;
use sndfile::SndFileIO;
use std::os::raw::c_void;

extern crate libflac_sys;
extern crate praw;
extern crate sndfile;

fn main() {
    // Convert all to flac
    let mut total_bufs: Vec<Vec<Vec<u8>>> = Vec::new();
    println!("Converting to flac");
    {
        let mut file = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path("./examples/song.flac")
            .unwrap();

        let mut file_len = file.len().unwrap() as usize * 2;
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        while file_len > 0 {
            total_bufs.push(Vec::new());
            let mut to_get_samples = 48000 * 2 * 10;
            if to_get_samples > file_len {
                to_get_samples = file_len;
            }

            file_len -= to_get_samples;

            let mut buf = Vec::with_capacity(to_get_samples);
            buf.resize(to_get_samples, 0.0);
            unsafe {
                let mut out_vector: Vec<u8> = Vec::new();
                let encoder = FLAC__stream_encoder_new();
                FLAC__stream_encoder_set_bits_per_sample(encoder, 32);
                FLAC__stream_encoder_set_channels(encoder, 2);
                FLAC__stream_encoder_set_compression_level(encoder, 8);
                FLAC__stream_encoder_set_sample_rate(encoder, 48000);

                let init_status = FLAC__stream_encoder_init_stream(
                    encoder,
                    Some(write_callback_encode),
                    None,
                    None,
                    None,
                    &mut out_vector as *mut _ as *mut _,
                );

                if init_status != FLAC__STREAM_ENCODER_INIT_STATUS_OK {
                    panic!("Failed to initialize the encoder");
                }

                file.read_to_slice(buf.as_mut_slice()).unwrap();
                // convert f32 to i32 from [-65536,65536]
                let buf = buf
                    .iter()
                    .map(|x| (x * 65536.0) as i32)
                    .collect::<Vec<i32>>();

                let encode_status = FLAC__stream_encoder_process_interleaved(
                    encoder,
                    buf.as_ptr(),
                    (buf.len() / 2) as u32,
                );

                if encode_status == FLAC__STREAM_ENCODER_WRITE_STATUS_OK as i32 {
                    panic!("Failed to encode the PCM data");
                }
                FLAC__stream_encoder_finish(encoder);
                FLAC__stream_encoder_delete(encoder);

                total_bufs.last_mut().unwrap().push(out_vector);
            }
        }
    }

    // Create praw writer
    println!("Writing to praw");
    {
        let mut writer = praw::PrawWriter::new(
            48000,
            vec![praw::Channels::Stereo],
            praw::AudioContainer::Flac,
        );
        for pack in total_bufs {
            writer.add_pack(vec![pack]);
        }

        // Write to file
        writer.save_to_file("./examples/song_flac.praw").unwrap();
    }
}

extern "C" fn write_callback_encode(
    _encoder: *const FLAC__StreamEncoder,
    buffer: *const u8,
    bytes: usize,
    _samples: u32,
    _current_frame: u32,
    client_data: *mut c_void,
) -> FLAC__StreamEncoderWriteStatus {
    let client_data = client_data as *mut Vec<u8>;
    unsafe {
        (*client_data).extend_from_slice(std::slice::from_raw_parts(buffer as *const u8, bytes));
    }
    FLAC__STREAM_ENCODER_WRITE_STATUS_OK
}
