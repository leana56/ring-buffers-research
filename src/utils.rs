use std::fs::OpenOptions;
use std::io::Write;
use std::io;


// =-= CL (Colors) =-= //
// - To add new colors, goto: https://talyian.github.io/ansicolors/
// - Use CL::End.get() at the end of the string to reset the color

pub enum CL {
    Pink,
    Purple,
    Green,
    LimeGreen,
    DullGreen,
    Blue,
    DimLightBlue,
    DullRed,
    Red,
    UrgentRed,
    PeachRed,
    Orange,
    Teal,
    DullTeal,
    Dull,
    End,
}

impl CL {
    pub fn get(&self) -> &'static str {
        match self {
            CL::Pink => "\x1b[38;5;165m",
            CL::Purple => "\x1b[38;5;135m",
            CL::Green => "\x1b[38;5;10m",
            CL::LimeGreen => "\x1b[38;5;154m",
            CL::DullGreen => "\x1b[38;5;29m",
            CL::Blue => "\x1b[38;5;27m",
            CL::DimLightBlue => "\x1b[38;5;159m",
            CL::DullRed => "\x1b[38;5;124m",
            CL::Red => "\x1b[38;5;1m",
            CL::UrgentRed => "\x1b[38;5;196m",
            CL::PeachRed => "\x1b[38;5;9m",
            CL::Orange => "\x1b[38;5;208m",
            CL::Teal => "\x1b[38;5;14m",
            CL::DullTeal => "\x1b[38;5;153m",
            CL::Dull => "\x1b[38;5;8m",
            CL::End => "\x1b[37m",
        }
    }
}


// =-= FileHandler =-= //
pub struct FileHandler {
    file: std::fs::File,
}

impl FileHandler {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file_path)?;
        Ok(Self { file })
    }

    pub fn write_line(&mut self, content: String) -> io::Result<()> {
        writeln!(self.file, "{}", content)
    }

    pub fn wipe_file(&mut self) -> io::Result<()> {
        self.file.set_len(0)
    }
}