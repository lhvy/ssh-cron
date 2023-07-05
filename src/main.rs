use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
use indicatif::ProgressBar;
use inquire::{InquireError, Select, Text};

#[tokio::main]
async fn main() -> Result<(), async_ssh2_tokio::Error> {
    let options: Vec<&str> = vec!["On (7am->5pm)", "On (7am->9pm)", "Off"];

    let ans: Result<&str, InquireError> =
        Select::new("What mode would you like?", options).prompt();
    let ans = ans.expect("Error selecting mode...");

    let hosts = [
        "tv1.i.ofgs.nsw.edu.au",
        "tv2.i.ofgs.nsw.edu.au",
        "tv3.i.ofgs.nsw.edu.au",
        "tv4.i.ofgs.nsw.edu.au",
        "kgtv1.i.ofgs.nsw.edu.au",
        "kgtv2.i.ofgs.nsw.edu.au",
        "kgtv3.i.ofgs.nsw.edu.au",
        "kgtv4.i.ofgs.nsw.edu.au",
        "k1tv1.i.ofgs.nsw.edu.au",
        "k1tv2.i.ofgs.nsw.edu.au",
        "k1tv3.i.ofgs.nsw.edu.au",
        "k1tv4.i.ofgs.nsw.edu.au",
        "y12commonroom.i.ofgs.nsw.edu.au",
        "l1tv1.i.ofgs.nsw.edu.au",
        "l1tv2.i.ofgs.nsw.edu.au",
    ];

    let password = Text::new("Enter the password").prompt();
    let password = password.expect("Something went wrong on input...");

    let auth_method = AuthMethod::with_password(&password);
    let pb = ProgressBar::new(hosts.len().try_into().unwrap());

    for &host in hosts.iter() {
        let client = Client::connect(
            (host, 22),
            "pi",
            auth_method.clone(),
            ServerCheckMethod::NoCheck,
        )
        .await;

        if client.is_err() {
            println!("ERROR: Cannot connect to {}!", host);
            continue;
        }
        let client = client?;

        let _ = client.execute("rm .display-test-abc").await?;
        // Don't check result, file may not exist.

        let result = client
            .execute("crontab -l | grep -v \"startDisplay\" >> .display-test-abc")
            .await?;
        if (result.output).starts_with("no crontab") {
            println!("ERROR: {} has no crontab, maybe make it manually...", host);
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
            "Off" => {}
            _ => unreachable!(),
        };

        let result = client.execute("cat .display-test-abc | crontab -").await?;
        assert_eq!(result.exit_status, 0);

        let _ = client.execute("rm .display-test-abc").await?;
        pb.inc(1);
    }
    pb.abandon();

    Ok(())
}
