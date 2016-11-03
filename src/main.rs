extern crate regex;
#[macro_use]
extern crate lazy_static;

use std::io::prelude::*;
use std::fs::File;
use std::str::FromStr;
use std::env;
use std::process;
use std::collections::HashMap;

use regex::Regex;

trait MachineCodeRepresentable {
    fn get_machine_code_string(&self) -> String;
}

const PREDEFINED_SYMBOLS: &'static [(&'static str, i16)] = &[("SP", 0),
                                                             ("LCL", 1),
                                                             ("ARG", 2),
                                                             ("THIS", 3),
                                                             ("THAT", 4),
                                                             ("SCREEN", 16384),
                                                             ("KBD", 24576),
                                                             ("R0", 0),
                                                             ("R1", 1),
                                                             ("R2", 2),
                                                             ("R3", 3),
                                                             ("R4", 4),
                                                             ("R5", 5),
                                                             ("R6", 6),
                                                             ("R7", 7),
                                                             ("R8", 8),
                                                             ("R9", 9),
                                                             ("R10", 10),
                                                             ("R11", 11),
                                                             ("R12", 12),
                                                             ("R13", 13),
                                                             ("R14", 14),
                                                             ("R15", 15)];

const C_INSTRUCTION_DESTINATION_REGEX_CLASS: &'static str = r#"(?:AMD|MD|AM|AD|M|D|A)"#;
const C_INSTRUCTION_OPERATION_REGEX_CLASS: &'static str = r#"A\+1|D\+1|D-1|A-1|D\+A|D-A|A-D|D&A|D\|A|M\+1|M-1|D\+M|D-M|M-D|D&M|D\|M|!M|-1|-M|!D|!A|-D|-A|0|1|M|D|A"#;
const C_INSTRUCTION_JUMP_CLASS: &'static str = r#"(?:JGT|JEQ|JGE|JLT|JNE|JLE|JMP)"#;
const IDENTIFIER_REGEX_STRING: &'static str = r#"(?:[:alpha:]|[_.&:\$])(?:[:alpha:]|[0-9]|[_.&:\$])*"#;

lazy_static! {
	static ref IDENTIFIER_REGEX: Regex = { Regex::new(&format!("^{}$", IDENTIFIER_REGEX_STRING)).unwrap() };
	static ref IMMEDIATE_REGEX: Regex = { Regex::new(r#"^[0-9]+$"#).unwrap() };

	static ref WHITESPACE_LINE_REGEX: Regex = { Regex::new(r#"^\s*$"#).unwrap()	};
	static ref COMMENT_LINE_REGEX: Regex = { Regex::new(r#"^//.*"#).unwrap() };
	static ref A_INSTRUCTION_REGEX: Regex = { Regex::new(r#"@(.+)"#).unwrap() };
	static ref C_INSTRUCTION_REGEX: Regex = {
		Regex::new(&format!(r#"^\s*(?:({destination_class})=)?({operation_class})(?:;({jump_class}))?(?:\s)*(?://.*)?$"#,
                           destination_class = C_INSTRUCTION_DESTINATION_REGEX_CLASS,
                           operation_class = C_INSTRUCTION_OPERATION_REGEX_CLASS,
                           jump_class = C_INSTRUCTION_JUMP_CLASS)).unwrap()
	};
    static ref LABEL_REGEX: Regex = { Regex::new(&format!(r#"\(({})\)"#, IDENTIFIER_REGEX_STRING)).unwrap() };
}

fn main() {
    let mut symbol_map = HashMap::with_capacity(PREDEFINED_SYMBOLS.len());
    for i in 0..PREDEFINED_SYMBOLS.len() {
        symbol_map.insert(PREDEFINED_SYMBOLS[i].0.to_string(), PREDEFINED_SYMBOLS[i].1);
    }

    let mut instruction_count = 0;
    let mut symbol_index = 16;

    let file_path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("No input file provided!");
            process::exit(0);
        }
    };

    let mut asm_file = match File::open(&file_path) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not open file at path '{}'", file_path);
            process::exit(0);
        }
    };

    let mut asm_string = String::new();
    asm_file.read_to_string(&mut asm_string).unwrap();
    let mut output_file = File::create("output.hack").unwrap();

    let line_iterator = asm_string.split("\n");
    let mut parsed_instructions = Vec::new();

    // assembler first pass. Parse lines, add symbols to table
    println!("Staring first pass:\n");
    for line in line_iterator.clone() {
        let line = line.trim();
        let parsed_line = line.parse::<ParsedLine>().unwrap();

        match parsed_line {
            ParsedLine::Label { ref label_identifier } => {
                symbol_map.insert(label_identifier.clone(), instruction_count);
            }
            ParsedLine::AInstruction { address_value } => {
                instruction_count += 1;
                parsed_instructions.push(ParsedLine::AInstruction { address_value: address_value });
            }
            ParsedLine::CInstruction { .. } => {
                instruction_count += 1;
                parsed_instructions.push(parsed_line);
            }
            ParsedLine::Whitespace => {}
            ParsedLine::Comment => {}
        };
    }

    println!("\nFirst pass completed\n\nSymbol Table:");
    for (identifier, value) in &symbol_map {
        println!("{:<50} {:>}", identifier, value);
    }

    // Second pass, generate machine code from parsed instructions, define variable symbols
    println!("\nStarting second pass\n:");
    for parsed_instruction in &parsed_instructions {
        match parsed_instruction {
            &ParsedLine::AInstruction { address_value: AddressValue::Immediate { ref value } } => {
                writeln!(output_file, "0{:0>15b}", value);
            }
            &ParsedLine::AInstruction { address_value: AddressValue::Symbol { ref identifier } } => {
                if !symbol_map.contains_key(identifier) {
                    println!("{:<50} {:>}", identifier, symbol_index);
                    symbol_map.insert(identifier.to_string(), symbol_index);
                    symbol_index += 1;
                }
                let address_value = symbol_map.get(identifier).unwrap();
                writeln!(output_file, "0{:0>15b}", address_value);
            }
            &ParsedLine::CInstruction { ref destination, ref operation, ref jump } => {
                writeln!(output_file, "111{}{}{}", 
                         operation.get_machine_code_string(),
                         destination.get_machine_code_string(), 
                         jump.get_machine_code_string());
            }
            &ParsedLine::Label { .. } => {
                panic!("A ParsedLine::Label has made it through to the second pass.")
            }
            &ParsedLine::Whitespace => {
                panic!("A ParsedLine::Whitespace has made through to the second pass.")
            }
            &ParsedLine::Comment => {
                panic!("A ParsedLine::Comment has made it through to the second pass.")
            }
        }
    }
}

#[derive(Debug)]
enum ParsedLine {
    Whitespace,
    Comment,
    AInstruction { address_value: AddressValue },
    CInstruction {
        destination: Destination,
        operation: Operation,
        jump: JumpCondition,
    },
    Label { label_identifier: String },
}

impl FromStr for ParsedLine {
    type Err = ();

    fn from_str(line: &str) -> Result<ParsedLine, ()> {
        let result = if WHITESPACE_LINE_REGEX.is_match(line) {
            ParsedLine::Whitespace
        } else if let Some(captures) = A_INSTRUCTION_REGEX.captures(line) {
            let address_value = captures.at(1).unwrap();
            ParsedLine::AInstruction {
                address_value: address_value.parse::<AddressValue>().unwrap(),
            }
        } else if let Some(captures) = C_INSTRUCTION_REGEX.captures(line) {
            ParsedLine::CInstruction {
                destination: captures.at(1).map_or(Destination::None,
                                                   |dest| dest.parse::<Destination>().unwrap()),
                operation: captures.at(2).unwrap().parse::<Operation>().unwrap(),
                jump: captures.at(3).map_or(JumpCondition::None,
                                            |jump| jump.parse::<JumpCondition>().unwrap()),
            }
        } else if let Some(captures) = LABEL_REGEX.captures(line) {
            ParsedLine::Label { label_identifier: captures.at(1).unwrap().to_string() }
        } else if COMMENT_LINE_REGEX.is_match(line) {
            ParsedLine::Comment
        } else {
            println!("Could not parse the following line:\n{}", line);
            process::exit(1);
        };

        Ok(result)
    }
}

#[derive(Debug)]
enum AddressValue {
    Immediate { value: i16 },
    Symbol { identifier: String },
}

impl FromStr for AddressValue {
    type Err = ();
    fn from_str(s: &str) -> Result<AddressValue, ()> {
        if IMMEDIATE_REGEX.is_match(s) {
            Ok(AddressValue::Immediate { value: s.parse::<i16>().unwrap() })
        } else if IDENTIFIER_REGEX.is_match(s) {
            Ok(AddressValue::Symbol { identifier: s.to_owned() })
        } else {
            println!("Malformed Address Value or Malformed RegEx, who knows which ...");
            process::exit(0);
        }
    }
}

#[derive(Debug, Clone)]
enum Destination {
    None,
    M,
    D,
    MD,
    A,
    AM,
    AD,
    AMD,
}

impl Destination {
    fn get_machine_code_string(&self) -> &str {
        match self {
            &Destination::M => "001",
            &Destination::D => "010",
            &Destination::MD => "011",
            &Destination::A => "100",
            &Destination::AM => "101",
            &Destination::AD => "110",
            &Destination::AMD => "111",
            &Destination::None => "000",
        }
    }
}

impl FromStr for Destination {
    type Err = ();

    fn from_str(s: &str) -> Result<Destination, ()> {
        match s {
            "M" => Ok(Destination::M),
            "D" => Ok(Destination::D),
            "MD" => Ok(Destination::MD),
            "A" => Ok(Destination::A),
            "AM" => Ok(Destination::AM),
            "AD" => Ok(Destination::AD),
            "AMD" => Ok(Destination::AMD),
            "" => Ok(Destination::None),
            _ => Result::Err(()),
        }
    }
}

#[derive(Debug, Clone)]
enum JumpCondition {
    None,
    JGT,
    JEQ,
    JGE,
    JLT,
    JNE,
    JLE,
    JMP,
}

impl MachineCodeRepresentable for JumpCondition {
    fn get_machine_code_string(&self) -> String {
        match self {
                &JumpCondition::JGT => "001",
                &JumpCondition::JEQ => "010",
                &JumpCondition::JGE => "011",
                &JumpCondition::JLT => "100",
                &JumpCondition::JNE => "101",
                &JumpCondition::JLE => "110",
                &JumpCondition::JMP => "111",
                &JumpCondition::None => "000",
            }
            .to_string()
    }
}

impl FromStr for JumpCondition {
    type Err = ();

    fn from_str(s: &str) -> Result<JumpCondition, ()> {
        match s {
            "JGT" => Ok(JumpCondition::JGT),
            "JEQ" => Ok(JumpCondition::JEQ),
            "JGE" => Ok(JumpCondition::JGE),
            "JLT" => Ok(JumpCondition::JLT),
            "JNE" => Ok(JumpCondition::JNE),
            "JLE" => Ok(JumpCondition::JLE),
            "JMP" => Ok(JumpCondition::JMP),
            "" => Ok(JumpCondition::None),
            _ => Result::Err(()),
        }
    }
}

#[derive(Debug, Clone)]
enum Operation {
    Zero,
    One,
    NegativeOne,
    D,
    A,
    NotD,
    NotA,
    NegD,
    NegA,
    DInc,
    AInc,
    DDec,
    ADec,
    DPlusA,
    DSubA,
    ASubD,
    DAndA,
    DOrA,
    M,
    NotM,
    NegM,
    MInc,
    MDec,
    DPlusM,
    DSubM,
    MSubD,
    DAndM,
    DOrM,
}

impl FromStr for Operation {
    type Err = ();

    fn from_str(s: &str) -> Result<Operation, ()> {
        match s {
            "0" => Ok(Operation::Zero),
            "1" => Ok(Operation::One),
            "-1" => Ok(Operation::NegativeOne),
            "D" => Ok(Operation::D),
            "A" => Ok(Operation::A),
            "!D" => Ok(Operation::NotD),
            "!A" => Ok(Operation::NotA),
            "-D" => Ok(Operation::NegD),
            "-A" => Ok(Operation::NegA),
            "D+1" => Ok(Operation::DInc),
            "A+1" => Ok(Operation::AInc),
            "D-1" => Ok(Operation::DDec),
            "A-1" => Ok(Operation::ADec),
            "D+A" => Ok(Operation::DPlusA),
            "D-A" => Ok(Operation::DSubA),
            "A-D" => Ok(Operation::ASubD),
            "D&A" => Ok(Operation::DAndA),
            "D|A" => Ok(Operation::DOrA),
            "M" => Ok(Operation::M),
            "!M" => Ok(Operation::NotM),
            "-M" => Ok(Operation::NegM),
            "M+1" => Ok(Operation::MInc),
            "M-1" => Ok(Operation::MDec),
            "D+M" => Ok(Operation::DPlusM),
            "D-M" => Ok(Operation::DSubM),
            "M-D" => Ok(Operation::MSubD),
            "D&M" => Ok(Operation::DAndM),
            "D|M" => Ok(Operation::DOrM),
            _ => Result::Err(()),
        }
    }
}

impl MachineCodeRepresentable for Operation {
    fn get_machine_code_string(&self) -> String {
        match self {
                &Operation::Zero => "0101010",
                &Operation::One => "0111111",
                &Operation::NegativeOne => "0111010",
                &Operation::D => "0001100",
                &Operation::A => "0110000",
                &Operation::NotD => "0001101",
                &Operation::NotA => "0110001",
                &Operation::NegD => "0001111",
                &Operation::NegA => "0110011",
                &Operation::DInc => "0011111",
                &Operation::AInc => "0110111",
                &Operation::DDec => "0001110",
                &Operation::ADec => "0110010",
                &Operation::DPlusA => "0000010",
                &Operation::DSubA => "0010011",
                &Operation::ASubD => "0000111",
                &Operation::DAndA => "0000000",
                &Operation::DOrA => "0010101",
                &Operation::M => "1110000",
                &Operation::NotM => "1110001",
                &Operation::NegM => "1110011",
                &Operation::MInc => "1110111",
                &Operation::MDec => "1110010",
                &Operation::DPlusM => "1000010",
                &Operation::DSubM => "1010011",
                &Operation::MSubD => "1000111",
                &Operation::DAndM => "1000000",
                &Operation::DOrM => "1010101",
            }
            .to_string()
    }
}
