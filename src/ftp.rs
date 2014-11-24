#![crate_name = "ftp"]
#![crate_type = "lib"]
#![feature(phase)]

extern crate regex;

#[phase(plugin)] extern crate regex_macros;

use std::io::{IoResult, TcpStream, BufferedReader};
use std::result::{Result};
use std::string::{String};

pub struct FTPStream {
	command_stream: TcpStream,
	pub host: &'static str,
	pub command_port: u16
}

impl FTPStream {
	
	pub fn connect(host: &'static str, port: u16) -> IoResult<FTPStream> {
		let connect_string = format!("{}:{}", host, port);
		let tcp_stream = try!(TcpStream::connect(connect_string.as_slice()));
		let mut ftp_stream = FTPStream {
			command_stream: tcp_stream,
			host: host,
			command_port: port
		};
		match ftp_stream.read_response(220) {
			Ok(_) => (),
			Err(e) => println!("{}", e)
		}
		Ok(ftp_stream)
	}

	pub fn login(&mut self, user: &str, password: &str) -> Result<(), String> {
		let user_command = format!("USER {}\r\n", user);
		let pass_command = format!("PASS {}\r\n", password);

		match self.command_stream.write_str(user_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(331) {
			Ok(_) => {

				match self.command_stream.write_str(pass_command.as_slice()) {
					Ok(_) => (),
					Err(_) => return Err(format!("Write Error"))
				}

				match self.read_response(230) {
					Ok(_) => Ok(()),
					Err(s) => Err(s)
				}
			},
			Err(s) => Err(s)
		}
	}

	pub fn change_dir(&mut self, path: &str) -> Result<(), String> {
		let cwd_command = format!("CWD {}\r\n", path);

		match self.command_stream.write_str(cwd_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(250) {
			Ok(_) => Ok(()),
			Err(e) => Err(e)
		}
	}

	pub fn change_dir_to_parent(&mut self) -> Result<(), String> {
		let cdup_command = format!("CDUP\r\n");

		match self.command_stream.write_str(cdup_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(250) {
			Ok(_) => Ok(()),
			Err(s) => Err(s)
		}
	}

	pub fn noop(&mut self) -> Result<(), String> {
		let noop_command = format!("NOOP\r\n");

		match self.command_stream.write_str(noop_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(200) {
			Ok(_) => Ok(()),
			Err(s) => Err(s)
		}
	}

	pub fn pasv(&mut self) -> Result<(int), String> {
		let pasv_command = format!("PASV\r\n");

		match self.command_stream.write_str(pasv_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		//PASV response format : 227 Entering Passive Mode (h1,h2,h3,h4,p1,p2).

		let response_regex = regex!(r"(.*)\(\d+,\d+,\d+,\d+,(\d+),(\d+)\)(.*)");

		match self.read_response(227) {
			Ok((_, line)) => {
				let caps = response_regex.captures(line.as_slice()).unwrap();
				let first_part_port: int = from_str(caps.at(2)).unwrap();
				let second_part_port: int = from_str(caps.at(3)).unwrap();
				Ok((first_part_port*256)+second_part_port)
			},
			Err(s) => Err(s)
		}
	}

	pub fn quit(&mut self) -> Result<(int, String), String> {
		let quit_command = format!("QUIT\r\n");
		
		match self.command_stream.write_str(quit_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(221) {
			Ok((code, message)) => Ok((code, message)),
			Err(message) => Err(message),
		}
	}

	pub fn retr(&mut self, file_name: &str) -> Result<BufferedReader<TcpStream>, String> {
		let retr_command = format!("RETR {}\r\n", file_name);

		let port = match self.pasv() {
			Ok(p) => p,
			Err(e) => return Err(e)
		};

		let connect_string = format!("{}:{}", self.host, port);
		let data_stream = BufferedReader::new(TcpStream::connect(connect_string.as_slice()).unwrap());

		match self.command_stream.write_str(retr_command.as_slice()) {
			Ok(_) => (),
			Err(_) => return Err(format!("Write Error"))
		}

		match self.read_response(150) {
			Ok(_) => Ok(data_stream),
			Err(e) => Err(e)
		}
	}

	//Retrieve single line response
	fn read_response(&mut self, expected_code: int) -> Result<(int, String), String> {
		//Carriage return
		let cr = 0x0d;
		//Line Feed
		let lf = 0x0a;
		let mut line_buffer: Vec<u8> = Vec::new();

		while line_buffer.len() < 2 || (line_buffer[line_buffer.len()-1] != lf && line_buffer[line_buffer.len()-2] != cr) {
				let byte_buffer: &mut [u8] = &mut [0];
				match self.command_stream.read(byte_buffer) {
					Ok(_) => {},
					Err(_) => return Err(format!("Error reading response")),
				}
				line_buffer.push(byte_buffer[0]);
		}

		let response = String::from_utf8(line_buffer).unwrap();
		let chars_to_trim: &[char] = &['\r', '\n'];
		let trimmed_response = response.as_slice().trim_chars(chars_to_trim);
    	let trimmed_response_vec: Vec<char> = trimmed_response.chars().collect();
    	if trimmed_response_vec.len() < 5 || trimmed_response_vec[3] != ' ' {
    		return Err(format!("Invalid response"));
    	}

    	let v: Vec<&str> = trimmed_response.splitn(1, ' ').collect();
    	let code: int = from_str(v[0]).unwrap();
    	let message = v[1];
    	if code != expected_code {
    		return Err(format!("Invalid response: {} {}", code, message))
    	}
    	Ok((code, String::from_str(message)))
	}
}