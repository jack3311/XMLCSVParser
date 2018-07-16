//
//  Description  :    An XML/CSV parser and formatter
//  Author       :    Jack Wilson
//  Mail         :    jack.wilson3311@gmail.com
//

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

trait Stack<T> {
    fn top(&mut self) -> &mut T;
}

impl<T> Stack<T> for Vec<T> {
    fn top(&mut self) -> &mut T {
        match self.len() {
            0 => panic!("Error: Vector does not have any elements!"),
            n => &mut self[n - 1],
        }
    }
}

#[derive(Debug)]
struct XMLNode {
    name: String,
    data: String,
    parent: Option<Rc<RefCell<XMLNode>>>,
    children: Vec<Rc<RefCell<XMLNode>>>,
}

impl XMLNode {
    fn new(name: String, parent: Option<Rc<RefCell<XMLNode>>>) -> XMLNode {
        XMLNode {
            name: name,
            data: String::new(),
            parent: parent,
            children: Vec::new(),
        }
    }

    fn get_path(&self) -> Vec<String> {
        let mut temp: Vec<String> = match self.parent {
            Some(ref node) => node.borrow().get_path(),
            None => Vec::new(),
        };
        temp.push(self.name.clone());
        temp
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
enum XMLTerm {
    OpeningTag(String),
    ClosingTag(String),
    Text(String),
    None,
}

impl XMLTerm {
    fn get_string(&mut self) -> Option<&mut String> {
        match *self {
            XMLTerm::OpeningTag(ref mut s) => Some(s),
            XMLTerm::ClosingTag(ref mut s) => Some(s),
            XMLTerm::Text(ref mut s) => Some(s),
            _ => None,
        }
    }
}

//Pushes a term to a vector of 'XMLTerm's
fn push_term(terms: &mut Vec<XMLTerm>, current_term: &mut XMLTerm) -> XMLTerm {
    //Copy term
    let mut new_term: XMLTerm = match *current_term {
        XMLTerm::OpeningTag(ref s) => XMLTerm::OpeningTag(s.trim().to_owned()),
        XMLTerm::ClosingTag(ref s) => XMLTerm::ClosingTag(s.trim().to_owned()),
        XMLTerm::Text(ref s) => XMLTerm::Text(s.trim().to_owned()),
        XMLTerm::None => XMLTerm::None,
    };

    //If the content of the term is non-empty, push it
    if new_term.get_string().unwrap_or(&mut "".to_owned()).len() > 0 {
        terms.push(new_term.clone());
    }

    XMLTerm::None
}

//Converts a read XML file into a vector of 'XMLTerm's
fn lexer(file_contents: String) -> Result<Vec<XMLTerm>, String> {
    let mut terms: Vec<XMLTerm> = Vec::new();
    let mut current_term = XMLTerm::None;
    let mut previous_char = '\0';
    let mut skip_until_next_line = false;

    //Read character-by-character
    for c in file_contents.chars() {
        //Ignore XML declaration thing
        if c == '\n' && skip_until_next_line {
            skip_until_next_line = false;
            previous_char = '\0';
            current_term = XMLTerm::None;
        }
        if skip_until_next_line { continue; }

        match c {
            '<' => {
                //Check if we should end a a text term
                match current_term {
                    XMLTerm::Text(_) => {
                        current_term = push_term(&mut terms, &mut current_term)
                    },
                    _ => {},
                }

                //Try create new opening tag
                match current_term {
                    XMLTerm::None => current_term = XMLTerm::OpeningTag(String::new()),
                    _ => return Err("Unexpected '<'".to_owned()),
                }
            },
            '>' => {
                //Try to push current tag
                match current_term {
                    XMLTerm::OpeningTag(_) => current_term = push_term(&mut terms, &mut current_term),
                    XMLTerm::ClosingTag(_) => current_term = push_term(&mut terms, &mut current_term),
                    _ => return Err("Unexpected '>'".to_owned()),
                }                
            },
            '/' if match current_term { XMLTerm::Text(_) => false, _ => true } => {
                //Try switch from opening tag to closing tag
                match current_term.clone() {
                    XMLTerm::OpeningTag(ref s) if previous_char == '<' => {
                        current_term = XMLTerm::ClosingTag(s.clone());
                    },
                    _ => return Err("Unexpected '/'".to_owned()),
                }
            },
            '?' => {
                //Ignore XML declaration thing
                skip_until_next_line = true;
            },
            _ => {
                //Create a new text element if we are outside of any elements
                match current_term {
                    XMLTerm::None /*if c != ' ' && c != '\t' && c != '\n'*/ => current_term = XMLTerm::Text(String::new()),
                    _ => {}
                }

                //Add character to current element content
                if let Some(s) = current_term.get_string() {
                    s.push(c);
                }
            },
        };

        previous_char = c;
    }

    Ok(terms)
}

//Converts a string of 'XMLTerm's into a XML tree
fn parser(terms: &Vec<XMLTerm>) -> Result<Rc<RefCell<XMLNode>>, String> {
    let root: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new("root".to_owned(), None)));
    let mut node_stack: Vec<Rc<RefCell<XMLNode>>> = Vec::new();

    node_stack.push(root.clone());

    for term in terms {
        match *term {
            XMLTerm::OpeningTag(ref s) => {
                //Create a new node
                let new_node: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new(s.to_owned(), Some(node_stack.top().clone()))));

                //Add it as a child of the current node
                node_stack.top().borrow_mut().children.push(new_node.clone());

                //Make this tag the current node
                node_stack.push(new_node.clone());
            },
            XMLTerm::ClosingTag(ref s) => {
                //Can only close the most recent opening tag!
                let expected_name = node_stack.top().borrow().name.clone();

                if *s == expected_name {
                    //Step back to this node's parent
                    node_stack.pop();
                }
                else {
                    return Err(format!("Unexpected closing tag. Found: {}, Expected: {}", s, expected_name));
                }
            },
            XMLTerm::Text(ref s) => {
                //Set the data of the current node
                node_stack.top().borrow_mut().data.push_str(s);
            }
            _ => {},
        }
    }
    
    Ok(root)
}

//Recursively converts an XML tree node into key/values in a map for CSV formatting
fn recursive_csv_format(node: Rc<RefCell<XMLNode>>, keymap: &mut HashMap<String, RefCell<Vec<String>>>, index: &mut usize) {
    let borrowed_node = node.borrow();

    if borrowed_node.children.len() == 0 { //If we are an 'end node'
        if borrowed_node.data.len() > 0 && borrowed_node.name.len() > 0 {
            //Create the node path
            let path = borrowed_node.get_path().join("/");

            //Ensure the key in the map
            if !keymap.contains_key(&path) {
                keymap.insert(path.clone(), RefCell::new(Vec::new()));
            }
            keymap[&path].borrow_mut().resize(*index, String::new());

            //Add this data to the map
            keymap[&path].borrow_mut().push(borrowed_node.data.clone());
        }

    }
    else {
        //Recurse to children
        for child in &borrowed_node.children {
            recursive_csv_format(child.clone(), keymap, index);
        }

        *index += 1;
    }
}

//Converts an XML tree into a CSV file string
fn csv_formatter(root: Rc<RefCell<XMLNode>>) -> String {
    let mut keymap: HashMap<String, RefCell<Vec<String>>> = HashMap::new();

    //The 'depth' of the csv
    let mut index: usize = 0;

    //Populate the map from the tree
    recursive_csv_format(root, &mut keymap, &mut index);

    //println!("{:?}", keymap);
    //println!("");

    let mut csv_string = String::new();

    //Push 'column' titles
    for key in keymap.keys() {
        if let Some(end_of_key) = key.split('/').last() {
            csv_string.push_str(end_of_key);
            csv_string.push(',');
        }
    }

    csv_string.pop();
    csv_string.push('\n');

    //Push row data
    for row in 0..index {
        for (_, vec) in &keymap {
            let borrowed_vec = vec.borrow();
            if borrowed_vec.len() >= row + 1 {
                csv_string.push_str(&borrowed_vec[row]);
                csv_string.push(',');
            }
        }

        csv_string.pop();
        csv_string.push('\n');
    }

    csv_string
}

//Converts a read CSV file into an XML tree
fn csv_parser(file_contents: String) -> Result<Rc<RefCell<XMLNode>>, String> {
    let mut keymap: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    let root: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new("root".to_owned(), None)));
    let root2: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new("root2".to_owned(), Some(root.clone()))));
    root.borrow_mut().children.push(root2.clone());

    let file_contents_trim = file_contents.trim();
    
    //Find all of the lines of the file
    let file_lines: Vec<&str> = file_contents_trim.split('\n').collect();

    if file_lines.len() <= 1 {
        return Err("No entries in CSV file".to_owned());
    }

    let mut read_keys: bool = false;

    for line in file_lines {
        //Find all of the comma-separated entries in the line
        let line_entries: Vec<&str> = line.split(',').collect();

        //If we have not yet read the 'column titles' row
        if !read_keys {
            //Read key names
            for entry in line_entries {
                keymap.push(entry.to_owned());
            }

            read_keys = true;
        }
        else {
            //Create a new row
            let mut row: Vec<String> = Vec::new();

            for entry in line_entries {
                //Add all of the entries to the row
                row.push(entry.to_owned());
            }

            //Add the row to the vec
            rows.push(row);
        }
    }


    //Convert to XML tree
    for row_index in 0..rows.len() {
        let row: &Vec<String> = &rows[row_index];
        let new_node: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new("element".to_owned(), Some(root2.clone()))));

        for key_index in 0..keymap.len() {
            if row.len() <= key_index {
                return Err(format!("Expected key {} for row {}", key_index, row_index));
            }

            let new_sub_node: Rc<RefCell<XMLNode>> = Rc::new(RefCell::new(XMLNode::new(keymap[key_index].clone(), Some(root2.clone()))));
            new_sub_node.borrow_mut().data = row[key_index].clone();
            new_node.borrow_mut().children.push(new_sub_node);
        }
        root2.borrow_mut().children.push(new_node);
    }

    //println!("{:?}", keymap);
    //println!("");
    //println!("{:?}", rows);

    Ok(root)
}

//Recursively converts an XML node yielding 'XMLTerm's
fn recursive_xml_reverse_parse(node: Rc<RefCell<XMLNode>>, mut terms: &mut Vec<XMLTerm>, depth: usize) {
    let node_borrowed = node.borrow();

    //Tabulate to depth
    terms.push(XMLTerm::Text("  ".repeat(depth)));

    //Write opening tag
    terms.push(XMLTerm::OpeningTag(node_borrowed.name.clone()));

    //Write data
    if node_borrowed.data.len() > 0 {
        terms.push(XMLTerm::Text(node_borrowed.data.clone()));
    }

    //If we span multiple lines, line break
    if node_borrowed.children.len() > 0 {
        terms.push(XMLTerm::Text("\n".to_owned()));
    }

    //Recurse for children
    for child in &node_borrowed.children {
        recursive_xml_reverse_parse(child.clone(), &mut terms, depth + 1);
    }

    //If we span multiple lines retabulate for closing tag
    if node_borrowed.children.len() > 0 {
        //Tabulate to depth
        terms.push(XMLTerm::Text("  ".repeat(depth)));
    }

    //Write closing tag
    terms.push(XMLTerm::ClosingTag(node_borrowed.name.clone()));

    //Line break
    terms.push(XMLTerm::Text("\n".to_owned()));
}

//Converts an XML tree into a vector of 'XMLTerm's
fn xml_reverse_parser(root: Rc<RefCell<XMLNode>>) -> Vec<XMLTerm> {
    let mut terms: Vec<XMLTerm> = Vec::new();

    if root.borrow().children.len() == 0 {
        panic!("Invalid XML tree");
    }

    //Recursively create terms from tree
    recursive_xml_reverse_parse(root.borrow().children[0].clone(), &mut terms, 0);

    terms
}

//Converts a vector of 'XMLTerm's into a XML file string
fn xml_formatter(terms: Vec<XMLTerm>) -> String {
    let mut xml_string = String::new();

    xml_string.push_str("<?xml version=\"1.0\"?>\n");

    for term in terms {
        if let Some(s) = match term {
            XMLTerm::OpeningTag(s) => Some(format!("<{}>", s)),
            XMLTerm::ClosingTag(s) => Some(format!("</{}>", s)),
            XMLTerm::Text(s) => Some(s),
            XMLTerm::None => None,
        } {
            xml_string.push_str(&s);
        }
    }

    xml_string
}


fn xml_to_csv(input_file: String, output_file: String) {
    let mut file_contents = String::new();

    {
        let mut file = match File::open(&input_file) {
            Err(e) => panic!("Could not open XML file {}: {}", input_file, e),
            Ok(file) => { println!("File opened successfully"); file },
        };

        match file.read_to_string(&mut file_contents) {
            Err(e) => panic!("Could not open XML file {}: {}", input_file, e),
            Ok(_) => println!("File read successfully"),
        }
    }

    let lexer_result = lexer(file_contents);

    let terms = match lexer_result {
        Err(error) => panic!("Error: {}", error),
        Ok(terms) => { println!("Completed lexical analysis"); terms },
    };

    let parser_result = parser(&terms);

    let root = match parser_result {
        Err(error) => panic!("Error: {}", error),
        Ok(root) => { println!("Completed parsing"); root },
    };

    let csv_result = csv_formatter(root);

    println!("Completed CSV formatting");
    
    match File::create(output_file) {
        Ok(mut file) => {
            match file.write(csv_result.as_bytes()) {
                Ok(_) => {
                    simple_message("Info", "CSV File written successfully");
                },
                Err(e) => {
                    simple_message("Error", &format!("Could not write to CSV file: {}", e));
                },
            };
        },
        Err(e) => {
            simple_message("Error", &format!("Could not create CSV file: {}", e));
        },
    };
}

fn csv_to_xml(input_file: String, output_file: String) {
    let mut file_contents = String::new();

    {
        let mut file = match File::open(&input_file) {
            Err(e) => panic!("Could not open CSV file {}: {}", input_file, e),
            Ok(file) => { println!("File opened successfully"); file },
        };

        match file.read_to_string(&mut file_contents) {
            Err(e) => panic!("Could not open CSV file {}: {}", input_file, e),
            Ok(_) => println!("File read successfully"),
        }
    }

    let parser_result = csv_parser(file_contents);

    let root = match parser_result {
        Ok(root) => root,
        Err(e) => panic!("Could not parse CSV file: {}", e),
    };

    let terms = xml_reverse_parser(root);

    println!("Completed XML reverse parsing");

    let xml_formatted = xml_formatter(terms);

    println!("Completed XML formatting");

    match File::create(output_file) {
        Ok(mut file) => {
            match file.write(xml_formatted.as_bytes()) {
                Ok(_) => {
                    simple_message("Info", "XML File written successfully");
                },
                Err(e) => {
                    simple_message("Error", &format!("Could not write to XML file: {}", e));
                },
            };
        },
        Err(e) => {
            simple_message("Error", &format!("Could not create XML file: {}", e));
        },
    };
}



//UI:

#[macro_use] extern crate native_windows_gui as nwg;

use nwg::{Event, Ui, simple_message, fatal_message, dispatch_events};
use nwg::constants::{FileDialogAction};

#[derive(Debug, Clone, Hash)]
pub enum AppId {
    // Controls
    MainWindow,
    InputFilePathInput, 
    OutputFilePathInput, 
    InputFileBrowseButton,
    OutputFileBrowseButton,
    XMLToCSVButton,
    CSVToXMLButton,
    FileDialogOpen,
    FileDialogSave,
    Label(u8),

    // Events
    XMLToCSVEvent,
    CSVToXMLEvent,
    InputFileBrowseEvent,
    OutputFileBrowseEvent,

    // Resources
    MainFont,
    TextFont
}

use AppId::*;


const WIDTH: u32 = 600;
const HEIGHT: u32 = 150;


nwg_template!(
    head: setup_ui<AppId>,
    controls: [
        (MainWindow, nwg_window!( title = "XML/CSV Parser - Jack Wilson 2017"; size = (WIDTH, HEIGHT) )),

        (Label(0), nwg_label!( parent = MainWindow; text = "Input File Path: "; position = (0, 0); size = (WIDTH / 2, 25); font = Some(TextFont) )),
        (InputFilePathInput, nwg_textinput!( parent = MainWindow; position = (0, 30); size = (WIDTH / 2, 22); font = Some(TextFont) )),
        (InputFileBrowseButton, nwg_button!( parent = MainWindow; text = "Browse..."; position = (0, 55); size = (WIDTH / 2, 25); font = Some(TextFont) )),

        (Label(1), nwg_label!( parent = MainWindow; text = "Output File Path: "; position = ((WIDTH / 2) as i32, 0); size = (WIDTH / 2, 25); font = Some(TextFont) )),
        (OutputFilePathInput, nwg_textinput!( parent = MainWindow; position = ((WIDTH / 2) as i32, 30); size = (WIDTH / 2, 22); font = Some(TextFont) )),
        (OutputFileBrowseButton, nwg_button!( parent = MainWindow; text = "Browse..."; position = ((WIDTH / 2) as i32, 55); size = (WIDTH / 2, 25); font = Some(TextFont) )),

        (XMLToCSVButton, nwg_button!( parent = MainWindow; text = "XML to CSV"; position = (0, (HEIGHT - 50) as i32); size = (WIDTH / 2, 50); font = Some(MainFont) )),
        (CSVToXMLButton, nwg_button!( parent = MainWindow; text = "CSV to XML"; position = ((WIDTH / 2) as i32, (HEIGHT - 50) as i32); size = (WIDTH / 2, 50); font = Some(MainFont) )),

        (FileDialogOpen, nwg_filedialog!(parent = Some(MainWindow); action = FileDialogAction::Open; filters = Some("Source Files(*.xml;*.csv)|Any(*.*)"))),
        (FileDialogSave, nwg_filedialog!(parent = Some(MainWindow); action = FileDialogAction::Save; filters = Some("Source Files(*.xml;*.csv)|Any(*.*)")))
    ];
    events: [
        (InputFileBrowseButton, InputFileBrowseEvent, Event::Click, |ui,_,_,_| {
            let (dialog, file_path) = nwg_get_mut!(ui; [
                (FileDialogOpen, nwg::FileDialog),
                (InputFilePathInput, nwg::TextInput)
            ]);

            if dialog.run() {
                file_path.set_text(&dialog.get_selected_item().unwrap());
            }
        }),
        (OutputFileBrowseButton, OutputFileBrowseEvent, Event::Click, |ui,_,_,_| {
            let (dialog, file_path) = nwg_get_mut!(ui; [
                (FileDialogSave, nwg::FileDialog),
                (OutputFilePathInput, nwg::TextInput)
            ]);

            if dialog.run() {
                file_path.set_text(&dialog.get_selected_item().unwrap());
            }
        }),


        (XMLToCSVButton, XMLToCSVEvent, Event::Click, |ui,_,_,_| {
            let input_file = nwg_get!(ui; (InputFilePathInput, nwg::TextInput));
            let output_file = nwg_get!(ui; (OutputFilePathInput, nwg::TextInput));
            
            let input_filename: String = input_file.get_text().trim().to_owned();
            let output_filename: String = output_file.get_text().trim().to_owned();

            if input_filename.len() == 0 {
                simple_message("Error", "Please select an input file!");
            }
            else if output_filename.len() == 0 {
                simple_message("Error", "Please select an output file!");
            }
            else
            {
                //XML TO CSV:
                xml_to_csv(input_filename, output_filename);
            }


        }),
        (CSVToXMLButton, CSVToXMLEvent, Event::Click, |ui,_,_,_| {
            let input_file = nwg_get!(ui; (InputFilePathInput, nwg::TextInput));
            let output_file = nwg_get!(ui; (OutputFilePathInput, nwg::TextInput));
            
            let input_filename: String = input_file.get_text().trim().to_owned();
            let output_filename: String = output_file.get_text().trim().to_owned();

            if input_filename.len() == 0 {
                simple_message("Error", "Please select an input file!");
            }
            else if output_filename.len() == 0 {
                simple_message("Error", "Please select an output file!");
            }
            else
            {
                //XML TO CSV:
                csv_to_xml(input_filename, output_filename);
            }
        })
    ];
    resources: [
        (MainFont, nwg_font!(family="Arial"; size=27)),
        (TextFont, nwg_font!(family="Arial"; size=17))
    ];
    values: []
);

fn main() {
    let app: Ui<AppId>;

    match Ui::new() {
        Ok(_app) => { app = _app; },
        Err(e) => { fatal_message("Fatal Error", &format!("{:?}", e) ); }
    }

    if let Err(e) = setup_ui(&app) {
        fatal_message("Fatal Error", &format!("{:?}", e));
    }

    dispatch_events();
}