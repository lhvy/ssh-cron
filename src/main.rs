use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
use indicatif::{MultiProgress, ProgressBar};
use inquire::{InquireError, Password, Select};
use linecount::count_lines;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), async_ssh2_tokio::Error> {
    let options: Vec<&str> = vec!["On (7am->5pm)", "On (7am->9pm)", "Always On", "Off"];

    let ans: Result<&str, InquireError> =
        Select::new("What mode would you like?", options).prompt();
    let ans = ans.expect("Error selecting mode...");

    let password = Password::new("Enter the password")
        .without_confirmation()
        .prompt();
    let password = password.expect("Something went wrong on input...");

    let auth_method = AuthMethod::with_password(&password);
    let n = count_lines(File::open("./hosts.txt").expect("Could not find file called hosts.txt"))
        .unwrap();

    let m = MultiProgress::new();
    let pb = m.add(ProgressBar::new(n.try_into().unwrap()));

    if let Ok(lines) = read_lines("./hosts.txt") {
        // Consumes the iterator, returns an (Optional) String
        for host in lines.flatten() {
            let client = Client::connect(
                (host.to_owned(), 22),
                "pi",
                auth_method.clone(),
                ServerCheckMethod::NoCheck,
            )
            .await;

            if client.is_err() {
                let _ = m.println(format!("ERROR: Cannot connect to {}!", host));
                continue;
            }
            let client = client?;

            let _ = client.execute("rm .display-test-abc").await?;
            // Don't check result, file may not exist.

            let result = client
                .execute("crontab -l | grep -v \"startDisplay\" >> .display-test-abc")
                .await?;
            if (result.output).starts_with("no crontab") {
                let _ = m.println(format!(
                    "ERROR: {} has no crontab, maybe make it manually...",
                    host
                ));
                continue;
            }

            let off_cron = "1 17,21 * * * export DISPLAY=:0 && /home/pi/startDisplaynight.sh";
            let result = client
                .execute(&format!("echo \"{}\" >> .display-test-abc", off_cron))
                .await?;
            assert_eq!(result.exit_status, 0);

            match ans {
                "On (7am->5pm)" => {
                    let result = client
                            .execute("echo \"*/5 7-16 * * mon-fri export DISPLAY=:0 && /home/pi/startDisplay.sh\" >> .display-test-abc")
                            .await?;
                    assert_eq!(result.exit_status, 0);
                }
                "On (7am->9pm)" => {
                    let result = client
                        .execute("echo \"*/5 7-20 * * mon-fri export DISPLAY=:0 && /home/pi/startDisplay.sh\" >> .display-test-abc")
                        .await?;
                    assert_eq!(result.exit_status, 0);
                }
                "Always On" => {
                    let result = client
                        .execute("echo \"*/5 * * * * export DISPLAY=:0 && /home/pi/startDisplay.sh\" >> .display-test-abc")
                        .await?;
                    assert_eq!(result.exit_status, 0);
                }
                "Off" => {
                    client
                        .execute("export DISPLAY=:0 && /home/pi/startDisplaynight.sh")
                        .await?;
                }
                _ => unreachable!(),
            };

            let result = client.execute("cat .display-test-abc | crontab -").await?;
            assert_eq!(result.exit_status, 0);

            let _ = client.execute("rm .display-test-abc").await?;
            pb.inc(1);
        }
    }

    pb.abandon();

    Ok(())
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
