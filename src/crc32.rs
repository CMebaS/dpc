use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Error, Read, Write};
use std::option::Option;
use std::path::Path;
use std::path::PathBuf;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use crc32fast::Hasher;
use itertools::Itertools;

pub trait CRC32 {
    fn hash(self: &Self, name: &[u8]) -> u32;
    fn generate_names(
        self: &Self,
        input: &mut dyn Read,
        output: &mut dyn Write,
        flush: bool,
        unsigned_option: bool,
        literal_option: bool,
    ) -> Result<(), Error> {
        let input_buffer = BufReader::new(input);
        let mut output_buffer = BufWriter::new(output);
        for line in input_buffer.lines() {
            let name = line?;
            let mut name_str = name.as_str();
            if !literal_option {
                name_str = name.trim();
            }

            let hash = self.hash(name_str.as_bytes());

            if unsigned_option {
                output_buffer.write(format!("{:?} \"{}\"\n", hash as u32, name_str).as_bytes())?;
            } else {
                output_buffer.write(format!("{:?} \"{}\"\n", hash as i32, name_str).as_bytes())?;
            }

            if flush {
                output_buffer.flush()?;
            }
        }

        Ok(())
    }

    fn generate_binary(
        self: &Self,
        input: &mut dyn Read,
        output: &mut dyn Write,
        unsigned_option: bool,
        offset: Option<usize>,
        length: Option<usize>,
    ) -> Result<(), Error> {
        let mut data: Vec<u8> = Vec::new();
        input.read_to_end(&mut data)?;

        let start = offset.unwrap_or(0);
        let end = length.unwrap_or(data.len());

        let hash = self.hash(&data[start..end]);

        if unsigned_option {
            output.write(format!("{:?}\n", hash as u32).as_bytes())?;
        } else {
            output.write(format!("{:?}\n", hash as i32).as_bytes())?;
        }

        Ok(())
    }
}

pub struct AsoboCRC32 {}

// const CRC32_POLYNOMIAL : u32 = 0x04C11DB7;
const CRC32_TABLE_SIZE: usize = 256;
const CRC32_TABLE: [u32; CRC32_TABLE_SIZE] = [
    0x00000000, 0x04C11DB7, 0x09823B6E, 0x0D4326D9, 0x130476DC, 0x17C56B6B, 0x1A864DB2,
    0x1E475005, //   0 [0x00 .. 0x07]
    0x2608EDB8, 0x22C9F00F, 0x2F8AD6D6, 0x2B4BCB61, 0x350C9B64, 0x31CD86D3, 0x3C8EA00A,
    0x384FBDBD, //   8 [0x08 .. 0x0F]
    0x4C11DB70, 0x48D0C6C7, 0x4593E01E, 0x4152FDA9, 0x5F15ADAC, 0x5BD4B01B, 0x569796C2,
    0x52568B75, //  16 [0x10 .. 0x17]
    0x6A1936C8, 0x6ED82B7F, 0x639B0DA6, 0x675A1011, 0x791D4014, 0x7DDC5DA3, 0x709F7B7A,
    0x745E66CD, //  24 [0x18 .. 0x1F]
    0x9823B6E0, 0x9CE2AB57, 0x91A18D8E, 0x95609039, 0x8B27C03C, 0x8FE6DD8B, 0x82A5FB52,
    0x8664E6E5, //  32 [0x20 .. 0x27]
    0xBE2B5B58, 0xBAEA46EF, 0xB7A96036, 0xB3687D81, 0xAD2F2D84, 0xA9EE3033, 0xA4AD16EA,
    0xA06C0B5D, //  40 [0x28 .. 0x2F]
    0xD4326D90, 0xD0F37027, 0xDDB056FE, 0xD9714B49, 0xC7361B4C, 0xC3F706FB, 0xCEB42022,
    0xCA753D95, //  48 [0x30 .. 0x37]
    0xF23A8028, 0xF6FB9D9F, 0xFBB8BB46, 0xFF79A6F1, 0xE13EF6F4, 0xE5FFEB43, 0xE8BCCD9A,
    0xEC7DD02D, //  56 [0x38 .. 0x3F]
    0x34867077, 0x30476DC0, 0x3D044B19, 0x39C556AE, 0x278206AB, 0x23431B1C, 0x2E003DC5,
    0x2AC12072, //  64 [0x40 .. 0x47]
    0x128E9DCF, 0x164F8078, 0x1B0CA6A1, 0x1FCDBB16, 0x018AEB13, 0x054BF6A4, 0x0808D07D,
    0x0CC9CDCA, //  72 [0x48 .. 0x4F]
    0x7897AB07, 0x7C56B6B0, 0x71159069, 0x75D48DDE, 0x6B93DDDB, 0x6F52C06C, 0x6211E6B5,
    0x66D0FB02, //  80 [0x50 .. 0x57]
    0x5E9F46BF, 0x5A5E5B08, 0x571D7DD1, 0x53DC6066, 0x4D9B3063, 0x495A2DD4, 0x44190B0D,
    0x40D816BA, //  88 [0x58 .. 0x5F]
    0xACA5C697, 0xA864DB20, 0xA527FDF9, 0xA1E6E04E, 0xBFA1B04B, 0xBB60ADFC, 0xB6238B25,
    0xB2E29692, //  96 [0x60 .. 0x67]
    0x8AAD2B2F, 0x8E6C3698, 0x832F1041, 0x87EE0DF6, 0x99A95DF3, 0x9D684044, 0x902B669D,
    0x94EA7B2A, // 104 [0x68 .. 0x6F]
    0xE0B41DE7, 0xE4750050, 0xE9362689, 0xEDF73B3E, 0xF3B06B3B, 0xF771768C, 0xFA325055,
    0xFEF34DE2, // 112 [0x70 .. 0x77]
    0xC6BCF05F, 0xC27DEDE8, 0xCF3ECB31, 0xCBFFD686, 0xD5B88683, 0xD1799B34, 0xDC3ABDED,
    0xD8FBA05A, // 120 [0x78 .. 0x7F]
    0x690CE0EE, 0x6DCDFD59, 0x608EDB80, 0x644FC637, 0x7A089632, 0x7EC98B85, 0x738AAD5C,
    0x774BB0EB, // 128 [0x80 .. 0x87]
    0x4F040D56, 0x4BC510E1, 0x46863638, 0x42472B8F, 0x5C007B8A, 0x58C1663D, 0x558240E4,
    0x51435D53, // 136 [0x88 .. 0x8F]
    0x251D3B9E, 0x21DC2629, 0x2C9F00F0, 0x285E1D47, 0x36194D42, 0x32D850F5, 0x3F9B762C,
    0x3B5A6B9B, // 144 [0x90 .. 0x97]
    0x0315D626, 0x07D4CB91, 0x0A97ED48, 0x0E56F0FF, 0x1011A0FA, 0x14D0BD4D, 0x19939B94,
    0x1D528623, // 152 [0x98 .. 0x9F]
    0xF12F560E, 0xF5EE4BB9, 0xF8AD6D60, 0xFC6C70D7, 0xE22B20D2, 0xE6EA3D65, 0xEBA91BBC,
    0xEF68060B, // 160 [0xA0 .. 0xA7]
    0xD727BBB6, 0xD3E6A601, 0xDEA580D8, 0xDA649D6F, 0xC423CD6A, 0xC0E2D0DD, 0xCDA1F604,
    0xC960EBB3, // 168 [0xA8 .. 0xAF]
    0xBD3E8D7E, 0xB9FF90C9, 0xB4BCB610, 0xB07DABA7, 0xAE3AFBA2, 0xAAFBE615, 0xA7B8C0CC,
    0xA379DD7B, // 176 [0xB0 .. 0xB7]
    0x9B3660C6, 0x9FF77D71, 0x92B45BA8, 0x9675461F, 0x8832161A, 0x8CF30BAD, 0x81B02D74,
    0x857130C3, // 184 [0xB8 .. 0xBF]
    0x5D8A9099, 0x594B8D2E, 0x5408ABF7, 0x50C9B640, 0x4E8EE645, 0x4A4FFBF2, 0x470CDD2B,
    0x43CDC09C, // 192 [0xC0 .. 0xC7]
    0x7B827D21, 0x7F436096, 0x7200464F, 0x76C15BF8, 0x68860BFD, 0x6C47164A, 0x61043093,
    0x65C52D24, // 200 [0xC8 .. 0xCF]
    0x119B4BE9, 0x155A565E, 0x18197087, 0x1CD86D30, 0x029F3D35, 0x065E2082, 0x0B1D065B,
    0x0FDC1BEC, // 208 [0xD0 .. 0xD7]
    0x3793A651, 0x3352BBE6, 0x3E119D3F, 0x3AD08088, 0x2497D08D, 0x2056CD3A, 0x2D15EBE3,
    0x29D4F654, // 216 [0xD8 .. 0xDF]
    0xC5A92679, 0xC1683BCE, 0xCC2B1D17, 0xC8EA00A0, 0xD6AD50A5, 0xD26C4D12, 0xDF2F6BCB,
    0xDBEE767C, // 224 [0xE0 .. 0xE7]
    0xE3A1CBC1, 0xE760D676, 0xEA23F0AF, 0xEEE2ED18, 0xF0A5BD1D, 0xF464A0AA, 0xF9278673,
    0xFDE69BC4, // 232 [0xE8 .. 0xEF]
    0x89B8FD09, 0x8D79E0BE, 0x803AC667, 0x84FBDBD0, 0x9ABC8BD5, 0x9E7D9662, 0x933EB0BB,
    0x97FFAD0C, // 240 [0xF0 .. 0xF7]
    0xAFB010B1, 0xAB710D06, 0xA6322BDF, 0xA2F33668, 0xBCB4666D, 0xB8757BDA, 0xB5365D03,
    0xB1F740B4, // 248 [0xF8 .. 0xFF]
];

impl CRC32 for AsoboCRC32 {
    fn hash(self: &Self, name: &[u8]) -> u32 {
        let mut hash: u32 = 0;

        for c in name {
            hash =
                (hash >> 8) ^ CRC32_TABLE[((c.to_ascii_lowercase() as u32 ^ hash) & 0xff) as usize];
        }

        hash
    }
}

pub struct AsoboCRC32Alt {}

impl CRC32 for AsoboCRC32Alt {
    fn hash(self: &Self, name: &[u8]) -> u32 {
        let mut hash: u32 = 0;

        for c in name {
            hash = (hash << 8)
                ^ CRC32_TABLE[((c.to_ascii_lowercase() as u32 ^ (hash >> 0x18)) & 0xff) as usize];
        }

        hash
    }
}

pub struct IEEECRC32 {}

impl CRC32 for IEEECRC32 {
    fn hash(self: &Self, name: &[u8]) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(name);
        hasher.finalize()
    }
}

pub struct CRC32SubCommand<'a> {
    algorithms: HashMap<&'a str, &'a dyn CRC32>,
}

impl CRC32SubCommand<'_> {
    pub fn new<'a>() -> CRC32SubCommand<'a> {
        let mut algorithms: HashMap<&str, &dyn CRC32> = HashMap::new();

        algorithms.insert("asobo", &AsoboCRC32 {});
        algorithms.insert("asobo_alt", &AsoboCRC32Alt {});
        algorithms.insert("ieee", &IEEECRC32 {});

        CRC32SubCommand { algorithms }
    }

    pub fn subcommand(self: &Self) -> App {
        SubCommand::with_name("crc32")
            .about("generate name files")
            .arg(
                Arg::with_name("BINARY")
                    .short("b")
                    .long("binary")
                    .conflicts_with("LITERAL")
                    .help("Treat the input as a a binary blob"),
            )
            .arg(
                Arg::with_name("OFFSET")
                    .short("s")
                    .long("offset")
                    .takes_value(true)
                    .requires("BINARY")
                    .help("Position to start hashing at"),
            )
            .arg(
                Arg::with_name("LENGTH")
                    .short("h")
                    .long("length")
                    .takes_value(true)
                    .requires("BINARY")
                    .help("Length of data to hash"),
            )
            .arg(
                Arg::with_name("INTERACTIVE")
                    .short("I")
                    .long("interactive")
                    .conflicts_with_all(&["INPUT", "OUTPUT"])
                    .help("Run the command in interactive mode"),
            )
            .arg(
                Arg::with_name("LITERAL")
                    .short("L")
                    .long("literal")
                    .help("Don't trim whitespace"),
            )
            .arg(
                Arg::with_name("ALGORITHM")
                    .short("a")
                    .long("algorithm")
                    .takes_value(true)
                    .required(true)
                    .possible_values(
                        self.algorithms
                            .keys()
                            .map(|x| x.clone())
                            .collect_vec()
                            .as_slice(),
                    )
                    .help("The crc32 algorithm to use"),
            )
            .arg(
                Arg::with_name("UNSIGNED")
                    .short("U")
                    .long("unsigned")
                    .help("Use unsigned values"),
            )
            .settings(&[AppSettings::ArgRequiredElseHelp])
    }

    pub fn execute(
        self: &Self,
        matches: &ArgMatches,
        subcommand_matches: &ArgMatches,
    ) -> Result<(), io::Error> {
        let binary_option = subcommand_matches.is_present("BINARY");
        let unsigned_option = subcommand_matches.is_present("UNSIGNED");
        let literal_option = subcommand_matches.is_present("LITERAL");
        let mut interactive_option = subcommand_matches.is_present("INTERACTIVE");

        let (mut input, mut output): (Box<dyn Read>, Box<dyn Write>) = if interactive_option {
            (Box::new(io::stdin()), Box::new(io::stdout()))
        } else {
            let input_path_string = matches.value_of_os("INPUT").unwrap();
            let input_path = Path::new(input_path_string);
            let output_writer: Box<dyn Write> = match matches.value_of_os("OUTPUT") {
                Some(output_path_string) => {
                    Box::new(File::create(PathBuf::from(output_path_string))?)
                }
                None => {
                    interactive_option = true;
                    Box::new(io::stdout())
                }
            };
            (Box::new(File::open(input_path)?), output_writer)
        };

        match subcommand_matches.value_of("ALGORITHM") {
            None => panic!("Algorithm is required"),
            Some(algorithm) => {
                if let Some(crc32_implementation) = self.algorithms.get(algorithm) {
                    if binary_option {
                        let offset = match subcommand_matches.value_of("OFFSET") {
                            None => None,
                            Some(off) => Some(off.parse::<usize>().unwrap()),
                        };
                        let length = match subcommand_matches.value_of("LENGTH") {
                            None => None,
                            Some(len) => Some(len.parse::<usize>().unwrap()),
                        };

                        crc32_implementation.generate_binary(
                            input.as_mut(),
                            output.as_mut(),
                            interactive_option,
                            offset,
                            length,
                        )?;
                    } else {
                        crc32_implementation.generate_names(
                            input.as_mut(),
                            output.as_mut(),
                            interactive_option,
                            unsigned_option,
                            literal_option,
                        )?;
                    }
                } else {
                    panic!("bad algorithm")
                }
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {}
