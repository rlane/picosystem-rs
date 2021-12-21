// File format:
// decompressed size: u16
// Repeated:
//   data length: u8
//   run length (repetitions of last value of data): u8
//   data: [u16]

#![no_std]

pub fn decompressed_size(input: &[u16]) -> u16 {
    if input.len() > 0 {
        input[0]
    } else {
        0
    }
}

fn ctrl_word(data_length: u8, run_length: u8) -> u16 {
    ((run_length as u16) << 8) | data_length as u16
}

pub fn decompress(input: &[u16], output: &mut [u16]) {
    let mut input_index: usize = 0;
    let mut output_index: usize = 0;
    let input_length = input.len();
    input_index += 1;

    while input_index < input_length {
        let ctrl = input[input_index];
        input_index += 1;
        let data_length = ctrl & 0xff;
        let run_length = ctrl >> 8;

        if data_length == 0 {
            output_index += run_length as usize;
        } else {
            for _ in 0..data_length {
                output[output_index] = input[input_index];
                output_index += 1;
                input_index += 1;
            }

            for _ in 0..run_length {
                output[output_index] = input[input_index - 1];
                output_index += 1;
            }
        }
    }
}

pub fn compress(input: &[u16], output: &mut [u16]) -> usize {
    let mut input_index: usize = 1;
    let mut output_index: usize = 0;
    let input_length = input.len();
    let mut data_start_index: usize = 0;
    let mut data_length: u8 = 1;
    let mut run_length: u8 = 0;

    let mut write = |v: u16| {
        output[output_index] = v;
        output_index += 1;
    };

    write(input_length as u16);

    if input_length == 0 {
        return output_index;
    }

    let mut last_value: u16 = input[0];
    while input_index < input_length {
        let value = input[input_index];
        input_index += 1;

        if value == last_value && run_length < 255 {
            run_length += 1;
        } else {
            /*
            if data_length >= 255 {
                write(ctrl_word(data_length, run_length));
                for i in 0..(data_length as usize) {
                    write(input[data_start_index + i]);
                }
                data_start_index = input_index - 1;
                data_length = 1;
            } else if run_length >= 3 {
                write(ctrl_word(data_length, run_length));
                for i in 0..(data_length as usize) {
                    write(input[data_start_index + i]);
                }
                data_start_index = input_index - 1;
                data_length = 1;
            } else {
                data_length += run_length;
            }
            last_value = value;
            run_length = 1;
            */
            if run_length >= 3 {
                write(ctrl_word(data_length, run_length));
                for i in 0..(data_length as usize) {
                    write(input[data_start_index + i]);
                }

                data_start_index = input_index - 1;
                data_length = 1;
                run_length = 0;
            } else {
                data_length += run_length + 1;
                run_length = 0;
            }
            last_value = value;
        }
    }

    if data_length > 0 {
        write(ctrl_word(data_length, run_length));
        for i in 0..(data_length as usize) {
            write(input[data_start_index + i]);
        }
    }

    output_index
}

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_decompressed_size() {
        assert_eq!(decompressed_size(&[]), 0);
        assert_eq!(decompressed_size(&[1]), 1);
    }

    #[test]
    fn test_decompress_skip() {
        let input = [16, ctrl_word(0, 16)];
        let mut output = [0; 16];
        decompress(&input, &mut output);
        assert_eq!(output, [0; 16]);
    }

    #[test]
    fn test_decompress_data_without_run() {
        let input = [2, ctrl_word(2, 0), 0xaa, 0xbb];
        let mut output = [0; 2];
        decompress(&input, &mut output);
        assert_eq!(output, [0xaa, 0xbb]);
    }

    #[test]
    fn test_decompress_data_with_run() {
        let input = [6, ctrl_word(2, 3), 0xaa, 0xbb];
        let mut output = [0; 5];
        decompress(&input, &mut output);
        assert_eq!(output, [0xaa, 0xbb, 0xbb, 0xbb, 0xbb]);
    }

    #[test]
    fn test_compress_empty() {
        let input = [];
        let mut output = [0; 100];
        let output_length = compress(&input, &mut output);
        assert_eq!(&output[0..output_length], [0]);
    }

    #[test]
    fn test_compress_just_runs() {
        let input = [0xaa, 0xaa, 0xaa, 0xaa, 0xbb, 0xbb, 0xbb, 0xbb];
        let mut output = [0; 100];
        let output_length = compress(&input, &mut output);
        assert_eq!(
            &output[0..output_length],
            [8, ctrl_word(1, 3), 0xaa, ctrl_word(1, 3), 0xbb]
        );
    }

    #[test]
    fn test_compress_data_without_run() {
        let input = [0xaa, 0xbb, 0xcc, 0xdd];
        let mut output = [0; 100];
        let output_length = compress(&input, &mut output);
        assert_eq!(
            &output[0..output_length],
            [4, ctrl_word(4, 0), 0xaa, 0xbb, 0xcc, 0xdd]
        );
    }

    #[test]
    fn test_max_run() {
        let input = [0xaa; 256];
        let mut output = [0; 100];
        let output_length = compress(&input, &mut output);
        assert_eq!(&output[0..output_length], [256, ctrl_word(1, 255), 0xaa]);
    }

    #[test]
    fn test_long_run() {
        let input = [0xaa; 257];
        let mut output = [0; 100];
        let output_length = compress(&input, &mut output);
        assert_eq!(
            &output[0..output_length],
            [257, ctrl_word(1, 255), 0xaa, 1, 0xaa]
        );
    }

    #[test]
    fn test_random() {
        let mut total_compressed_size = 0;
        const M: usize = 100;
        for _ in 0..M {
            let mut rng = rand::thread_rng();
            const N: usize = 16384;
            let mut input = [0; N];
            let mut compressed = [0; N * 2 + 1];
            let mut output = [0; N];
            for i in 0..N {
                if i == 0 || rng.gen::<f32>() < 0.5 {
                    input[i] = 100 + (rng.gen::<u16>() % 10);
                } else {
                    input[i] = input[i - 1];
                }
            }
            let output_length = compress(&input, &mut compressed);
            println!("input:      {:?}", input);
            println!("compressed: {:?}", compressed);
            decompress(&compressed[0..output_length], &mut output);
            println!("output:     {:?}", output);
            assert_eq!(input, output);
            total_compressed_size += output_length;
        }
        println!("average compressed size: {}", total_compressed_size / M);
    }
}
