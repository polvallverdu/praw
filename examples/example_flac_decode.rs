use core::slice;
use libflac_sys::*;
use sndfile::SndFileIO;

extern crate libflac_sys;
extern crate praw;
extern crate sndfile;

struct Data {
    flac_data: Vec<u8>,
    pcm_data: Vec<i32>,
}

fn main() {
    // Read praw file
    println!("Reading and decoding praw");
    {
        let mut flac_data = Data {
            flac_data: vec![], // your flac data here
            pcm_data: vec![],
        };
        let mut reader = praw::PrawFileReader::load("./examples/song_flac.praw").unwrap();
        reader.go_to_pack(19).unwrap();
        //reader.go_to_pack(0).unwrap();

        let mut decoded_file = sndfile::OpenOptions::WriteOnly(sndfile::WriteOptions::new(
            sndfile::MajorFormat::WAV,
            sndfile::SubtypeFormat::PCM_32,
            sndfile::Endian::File,
            48000,
            2,
        ))
        .from_path("./examples/decoded_flac.wav")
        .unwrap();

        for _ in reader.current_pack..reader.packs_num {
            {
                let allpack = reader.read_pack().unwrap();
                let pack = &allpack[0];
                flac_data.flac_data = pack.data[0].clone();
                println!(
                    "Reading packet num {}/{}",
                    reader.current_pack, reader.reader.packs_num
                );
            }

            unsafe {
                let decoder = FLAC__stream_decoder_new();
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
                    _ => panic!(
                        "Failed to initialize the decoder. Status: {:?}",
                        decoder_status
                    ),
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

                decoded_file.write_from_slice(f32data.as_slice()).unwrap();
            }
        }
    }
}

unsafe extern "C" fn read_callback_decode(
    _decoder: *const FLAC__StreamDecoder,
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
    _decoder: *const FLAC__StreamDecoder,
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

unsafe extern "C" fn error_callback_decode(
    _decoder: *const FLAC__StreamDecoder,
    status: FLAC__StreamDecoderErrorStatus,
    _client_data: *mut std::ffi::c_void,
) {
    let error_message = match status {
        FLAC__STREAM_DECODER_ERROR_STATUS_LOST_SYNC => "An error occurred while decoding the stream. The decoder is now in the 'lost sync' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_BAD_HEADER => "An error occurred while decoding the stream. The decoder is now in the 'bad header' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_FRAME_CRC_MISMATCH => "An error occurred while decoding the stream. The decoder is now in the 'frame crc mismatch' state.",
        FLAC__STREAM_DECODER_ERROR_STATUS_UNPARSEABLE_STREAM => "An error occurred while decoding the stream. The decoder is now in the 'unparseable stream' state.",
        _ => "An unknown error occurred while decoding the stream.",
    };
    println!("Error: {}", error_message);
}
