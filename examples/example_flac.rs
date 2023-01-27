use core::slice;
use libflac_sys::*;
use sndfile::SndFileIO;
use std::os::raw::c_void;

extern crate claxon;
extern crate libflac_sys;
extern crate praw;
extern crate sndfile;

struct Data {
    flac_data: Vec<u8>,
    pcm_data: Vec<i32>,
}

fn main() {
    // Convert all to flac
    let mut total_bufs: Vec<Vec<Vec<u8>>> = Vec::new();
    println!("Converting to flac");
    {
        let mut file = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path("./examples/song.flac")
            .unwrap();

        let mut file_len = file.len().unwrap() as usize*2;
        let mut done = false;
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

                let len = file.read_to_slice(buf.as_mut_slice()).unwrap();
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

    // Read praw file
    println!("Reading and decoding praw");
    {
        let mut flac_data = Data {
            flac_data: vec![], // your flac data here
            pcm_data: vec![],
        };
        let mut reader = praw::PrawFileReader::load("./examples/song_flac.praw").unwrap();

        let mut decoded_file = sndfile::OpenOptions::WriteOnly(sndfile::WriteOptions::new(
            sndfile::MajorFormat::FLAC,
            sndfile::SubtypeFormat::PCM_16,
            sndfile::Endian::File,
            48000,
            2,
        ))
        .from_path("./examples/decoded_flac.flac")
        .unwrap();

        for _ in 0..reader.packs_num {
            {
                let allpack = reader.read_pack();
                let pack = &allpack[0];
                flac_data.flac_data = pack.data[0].clone();
                println!("Reading packet num {}", reader.reader.current_pack);
            }

            unsafe {
                let mut decoder = FLAC__stream_decoder_new();
                FLAC__stream_decoder_set_metadata_ignore_all(decoder);
                let decoder_status = FLAC__stream_decoder_init_stream(
                    decoder,
                    Some(read_callback_decode),
                    None,
                    None,
                    None,
                    None,
                    Some(write_callback_decode),
                    None,
                    Some(error_callback_decode),
                    &mut flac_data as *mut _ as *mut _,
                );

                match decoder_status {
                    FLAC__STREAM_DECODER_INIT_STATUS_OK => {}
                    _ => panic!("Failed to initialize the decoder. Status: {:?}", decoder_status),
                }

                //FLAC__stream_decoder_process_until_end_of_metadata(decoder);
                //flac_data.pcm_data.clear();
                FLAC__stream_decoder_process_until_end_of_stream(decoder);
                flac_data.flac_data.clear();

                // convert from i32 [-65536,65536] to f32 [-1.0,1.0]
                let f32data = flac_data
                    .pcm_data
                    .iter()
                    .map(|x| (*x as f32) / 65536.0)
                    .collect::<Vec<f32>>();
                flac_data.pcm_data.clear();

                decoded_file
                    .write_from_slice(f32data.as_slice())
                    .unwrap();
            }
        }
    }
}

extern "C" fn write_callback_encode(
    encoder: *const FLAC__StreamEncoder,
    buffer: *const u8,
    bytes: usize,
    samples: u32,
    current_frame: u32,
    client_data: *mut c_void,
) -> FLAC__StreamEncoderWriteStatus {
    let client_data = client_data as *mut Vec<u8>;
    unsafe {
        (*client_data).extend_from_slice(std::slice::from_raw_parts(buffer as *const u8, bytes));
    }
    FLAC__STREAM_ENCODER_WRITE_STATUS_OK
}

// extern "C" fn seek_callback(
//     _encoder: *mut FLAC__StreamEncoder,
//     _absolute_byte_offset: u64,
//     client_data: *mut c_void,
// ) -> FLAC__StreamEncoderSeekStatus {
//     let client_data = client_data as *mut Vec<u8>;
//     unsafe {
//         (*client_data).clear();
//     }
//     FLAC__STREAM_ENCODER_SEEK_STATUS_OK
// }

// extern "C" fn tell_callback(
//     _encoder: *mut FLAC__StreamEncoder,
//     _absolute_byte_offset: *mut u64,
//     client_data: *mut c_void,
// ) -> FLAC__StreamEncoderTellStatus {
//     let client_data = client_data as *mut Vec<u8>;
//     unsafe {
//         *_absolute_byte_offset = (*client_data).len() as u64;
//     }
//     FLAC__STREAM_ENCODER_TELL_STATUS_OK
// }

unsafe extern "C" fn read_callback_decode(
    decoder: *const FLAC__StreamDecoder,
    buffer: *mut FLAC__byte,
    bytes: *mut usize,
    client_data: *mut std::ffi::c_void,
) -> FLAC__StreamDecoderReadStatus {
    let data = client_data as *mut Data;
    let data = &mut *data;

    if *bytes > 0 {
        if *bytes > data.flac_data.len() {
            *bytes = data.flac_data.len();
        }

        if *bytes == 0 {
            return FLAC__STREAM_DECODER_READ_STATUS_END_OF_STREAM;
        }

        let buffer = slice::from_raw_parts_mut(buffer, *bytes as usize);
        let flac_data = &data.flac_data[..*bytes as usize];
        buffer.copy_from_slice(flac_data);
        data.flac_data.drain(..*bytes as usize);

        FLAC__STREAM_DECODER_READ_STATUS_CONTINUE
    } else {
        FLAC__STREAM_DECODER_READ_STATUS_ABORT
    }
}

unsafe extern "C" fn write_callback_decode(
    decoder: *const FLAC__StreamDecoder,
    frame: *const FLAC__Frame,
    buffer: *const *const FLAC__int32,
    client_data: *mut std::ffi::c_void,
) -> FLAC__StreamDecoderWriteStatus {
    let data = client_data as *mut Data;
    let data = &mut *data;

    let channels = (*frame).header.channels as usize;
    let blocksize = (*frame).header.blocksize as usize;

    // buffer to slice i32
    let buffer = slice::from_raw_parts(buffer, channels);
    let buffer = buffer
        .iter()
        .map(|x| slice::from_raw_parts(*x, blocksize))
        .collect::<Vec<&[i32]>>();

    for i in 0..blocksize {
        for j in 0..channels {
            data.pcm_data.push(buffer[j][i]);
        }
    }

    FLAC__STREAM_DECODER_WRITE_STATUS_CONTINUE
}

unsafe extern "C" fn error_callback_decode(decoder: *const FLAC__StreamDecoder, status: FLAC__StreamDecoderErrorStatus, client_data: *mut std::ffi::c_void) {
    let error_message = match status {
        FLAC__STREAM_DECODER_ERROR_STATUS_LOST_SYNC => "An error occurred while decoding the stream. The decoder is now in the 'lost sync' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_BAD_HEADER => "An error occurred while decoding the stream. The decoder is now in the 'bad header' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_FRAME_CRC_MISMATCH => "An error occurred while decoding the stream. The decoder is now in the 'frame crc mismatch' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_UNPARSEABLE_STREAM => "An error occurred while decoding the stream. The decoder is now in the 'unparseable stream' state.",
        _ => "An unknown error occurred while decoding the stream.",
    };
    println!("Error: {}", error_message);
}

