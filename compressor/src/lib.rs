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
            if run_length >= 3 || data_length >= 255 {
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
    fn test_long_data() {
        let mut input = [0u16; 257];
        for i in 0..257 {
            input[i] = i as u16;
        }
        let mut output = [0; 1000];
        let output_length = compress(&input, &mut output);
        #[rustfmt::skip]
        assert_eq!(
            &output[0..output_length],
            [ 257, ctrl_word(255, 0), 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, ctrl_word(2, 0), 255, 256 ]
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
