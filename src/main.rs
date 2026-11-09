use std::io::Write;
use std::path::Path;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::process::Stdio;

struct Terminal{
    cwd: Box<Path>,
    exited: bool
}

impl Terminal{
    fn new() -> Self {
        Self {
            cwd:
                std::env::current_dir()
                    .expect("???")
                    .as_path()
                    .into(),
            exited: false
        }
    }
    fn chdir(
        &mut self, path: &str
    ){
        let _bind = self.cwd.join(path);
        let _res = _bind.as_path();

        if _res.is_dir(){
            self.cwd = _res.into();
        }
    }

    fn combine_strings(
        &mut self, args: Vec<String>
    ) -> Vec<String> {
        let mut combined = vec![];
        let mut in_string = String::new();

        for a in args{
            if in_string.len()!=0 && a.ends_with("\""){
                combined.push(in_string.clone());
                continue;
            }
            if a.starts_with("\""){
                in_string.push_str(&a[1..]);
            }else{
                if a.contains("\"") && in_string.len()!=0{
                    panic!("wtf? quotes must always be at the end of arguments!");
                }
                if in_string.len()!=0{
                    in_string+=&a;
                }else{
                    combined.push(a);
                }
            }
        }
        combined
    }

    fn prompt(
        &mut self
    ){
        print!(
            "\x1b[1m\x1b[32m{}\x1b[94m $ \x1b[0m",
            self.get_short_path(
                &fs::canonicalize(
                    self.cwd.clone()
                ).expect("the hell?")
            )
        );
    }

    fn spawn(
        &self, cmd: String, args: Vec<String>
    ) -> Result<(), Box<dyn std::error::Error>>{
        let mut child = unsafe {
            Command::new(&cmd)
                .args(&args)
                .current_dir(self.cwd.clone())
                .env("CLICOLOR_FORCE", "1")
                .env("FORCE_COLOR", "1")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .pre_exec(|| {
                    libc::setpgid(0,0);
                    Ok(())
                })
                .spawn()?
        };
        
        let term = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;


        loop{
            if let Some(_) = child.try_wait()?{
                break Ok(());
            }
            if term.load(Ordering::Relaxed) {
                println!();
                child.kill()?;

                let _ = child.wait()?;
                break Ok(());
            }

            thread::sleep(
                Duration::from_millis(50)
            );
        }
    }

    fn execute(
        &mut self, input: String
    ){
        let mut _s: Vec<String> = input.split(" ")
            .map(String::from)
            .collect();
        _s = self.combine_strings(_s);

        let cmd = &_s[0];
        let args = &mut (_s[1..].to_vec());

        if cmd == "cd"{
            if args.len()!=0{
                self.chdir(&args[0]);
            }
        }
        else if cmd == "exit"{
            self.exited=true;
            println!("Exiting..");
            return;
        }
        else{
            if cmd=="ls"{
                args.insert(0, "--color=auto".to_string());
            }
            let _ = self.spawn(cmd.to_string(), args.to_vec());
        }
    }

    fn get_short_path(&self, path: &Path) -> String {
        if let Some(home_dir) = dirs::home_dir() {
            if path == home_dir {
                return "~".to_string();
            } else if let Ok(stripped) = path.strip_prefix(&home_dir) {
                return format!("~/{}", stripped.display());
            }
        }
        path.display().to_string()
    }

}



fn main() {
    let mut t = Terminal::new();
    let mut buff = String::new();
    while !t.exited {
        t.prompt();
        let _ = std::io::stdout().flush();
        let _ = std::io::stdin().read_line(&mut buff);

        let _input = buff.trim();

        t.execute(_input.to_string());
        buff.clear();
    }
}
