use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

fn explore(folder: PathBuf, tx: mpsc::Sender<(String, PathBuf)>) {
	for path in std::fs::read_dir(&folder).unwrap() {
		let item_tx = mpsc::Sender::clone(&tx);
		let p = path.unwrap();
		if p.path().is_file() {
			std::thread::spawn(move || {
				let digest = format!("{:x}", md5::compute(std::fs::read(p.path()).unwrap()));
				let full_file_path = p.path();

				item_tx.send((digest, full_file_path)).unwrap();
			});
		} else if p.path().is_dir() {
			let child_path = PathBuf::new().join(&folder).join(&p.file_name());
			std::thread::spawn(|| explore(child_path, item_tx));
		}
	}
}

fn main() {
	let app = clap::App::new(env!("CARGO_PKG_NAME"))
		.version(env!("CARGO_PKG_VERSION"))
		.subcommand(
			clap::SubCommand::with_name("list")
				.about("Search and list all duplicates in JSON format")
				.arg(
					clap::Arg::with_name("PATH")
						.required(false)
						.help("folder where to search")
						.takes_value(true)
						.default_value("."),
				),
		)
		.subcommand(
			clap::SubCommand::with_name("resolve")
				.about("Search duplicates and ask which one to move in the trash bin")
				.arg(
					clap::Arg::with_name("PATH")
						.required(false)
						.help("folder where to search")
						.takes_value(true)
						.default_value("."),
				),
		);

	let mut help_text = Vec::new();
	app.write_help(&mut help_text).unwrap();

	let matches = app.get_matches();

	if let Some(subcommand) = matches.subcommand_matches("list") {
		let target = subcommand.value_of("PATH").unwrap();
		let mut result: HashMap<String, Vec<PathBuf>> = HashMap::new();
		let (tx, rx) = mpsc::channel();
		let child_target = String::from(target);
		std::thread::spawn(move || explore(std::path::PathBuf::new().join(&child_target), tx));

		for (digest, item) in rx {
			match result.get_mut(&digest) {
				Some(items) => {
					items.push(item);
				}
				None => {
					result.insert(digest, vec![item]);
				}
			}
		}

		let filter: HashMap<&String, &Vec<PathBuf>> =
			result.iter().filter(|e| e.1.len() > 1).collect();

		println!("{}", serde_json::to_string(&filter).unwrap());
	} else if let Some(subcommand) = matches.subcommand_matches("resolve") {
		let target = subcommand.value_of("PATH").unwrap();
		let mut result: HashMap<String, Vec<PathBuf>> = HashMap::new();
		let (tx, rx) = mpsc::channel();
		let child_target = String::from(target);
		std::thread::spawn(move || explore(std::path::PathBuf::new().join(&child_target), tx));

		for (digest, item) in rx {
			match result.get_mut(&digest) {
				Some(items) => {
					items.push(item);
				}
				None => {
					result.insert(digest, vec![item]);
				}
			}
		}

		let mut counter = 1;
		for (digest, items) in result.iter().filter(|e| e.1.len() > 1) {
			println!("\n--- {} ({} of {})", digest, counter, result.len());
			let mut i = 1;
			for item in items {
				println!("#{} : {}", i, item.display());
				i = i + 1;
			}

			let mut go_to_next = false;
			while !go_to_next {
				go_to_next = true;
				println!("Which numbers to move in the trash ? (join them with coma «,»)");
				let mut targets = String::new();
				std::io::stdin().read_line(&mut targets).unwrap();

				if !targets.trim().is_empty() {
					for target in targets.trim().split(",") {
						let i = target.trim().parse::<usize>();

						match i {
							Ok(i) => {
								if i > 0 && i <= items.len() {
									let target = items.get(i - 1).unwrap();
									if let Err(e) = trash::remove(target) {
										println!(
											"Error while move in the trash file «{}» : {:?}",
											target.display(),
											e
										);
									}
								} else {
									println!("Number «{}» is out of range", i);
									go_to_next = false;
								}
							}
							Err(e) => {
								println!("Error while parsing number «{}» : {}", target, e);
								go_to_next = false;
							}
						}
					}
				} else {
					println!("No one has been deleted.");
				}
			}

			counter = counter + 1;
		}
	} else {
		println!("{}", std::str::from_utf8(&help_text).unwrap());
	}
}
