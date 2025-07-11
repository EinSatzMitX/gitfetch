#![allow(dead_code)]
use image::ImageReader;
use octocrab::{Octocrab, models::SimpleUser};

use clap::{ArgAction, Parser};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::HashMap, env};
use viuer::{Config, print_from_file};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContributionDays {
    date: String,
    contribution_count: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Weeks {
    contribution_days: Vec<ContributionDays>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContributionCalendar {
    weeks: Vec<Weeks>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContributionsCollection {
    contribution_calendar: ContributionCalendar,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UserData {
    login: String,         // maps from `login`
    name: Option<String>,  // `name` can be null
    email: Option<String>, // likewise
    bio: Option<String>,
    company: Option<String>,
    location: Option<String>,
    website_url: Option<String>,
    twitter_username: Option<String>,

    avatar_url: String,
    contributions_collection: ContributionsCollection,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GraphqlData {
    user: UserData,
}

#[derive(Deserialize, Debug)]
struct GraphqlResponse {
    data: GraphqlData,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    // personal github access token
    #[arg(short, long, action = ArgAction::Set)]
    token: Option<String>,

    // Github username to fetch
    #[arg(short = 'u', long = "user", action = ArgAction::Set, required = true)]
    username: Option<String>,
}

fn print_usage() {
    println!("Usage: gitfetch API_TOKEN");
}

fn sparkline(data: &[u32]) -> String {
    let max = *data.iter().max().unwrap_or(&1) as f32;
    data.iter()
        .map(|&v| {
            let idx = ((v as f32 / max) * 7.0).round() as usize;
            ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][idx.min(7)]
        })
        .collect()
}

/* Sparkline representation, but it scales logarithmically */
fn sparkline_log(data: &[u32]) -> String {
    // Unicode blocks for height
    const BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

    // GitHub contribution colors (hex → RGB)
    const COLORS: [(u8, u8, u8); 5] = [
        (33, 110, 57),   // level 4: darker green
        (48, 161, 78),   // level 3: dark green
        (64, 196, 99),   // level 2: medium green
        (155, 233, 168), // level 1: light green
        (235, 237, 240), // level 0: very light gray (no contributions)
    ];

    // Precompute ln(max+1)
    let max_ln = (*data.iter().max().unwrap_or(&0) as f32 + 1.0).ln();

    data.iter()
        .map(|&v| {
            // log‐scale fraction in [0,1]
            let frac = (v as f32 + 1.0).ln() / max_ln;
            // height index based on frac
            let h = (frac * 7.0).round().clamp(0.0, 7.0) as usize;
            // color level 0–4
            let lvl = (frac * 4.0).round().clamp(0.0, 4.0) as usize;
            let (r, g, b) = COLORS[lvl];
            // ANSI 24‐bit color: set fg to (r,g,b), print block, reset
            format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, BLOCKS[h])
        })
        .collect()
}

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    let args = CliArgs::parse();
    let gh;
    if let Some(tok) = &args.token {
        println!("got a token with length: {}", tok.len());
        gh = Octocrab::builder().personal_token(tok.clone()).build()?;
    } else {
        println!("No token provided, you may be rate limited!");
        gh = Octocrab::builder().build()?;
    }
    let query = r#"
      query($login: String!) {
        user(login: $login) {
          avatarUrl(size: 256)
          login
          name
          email
          bio
          company
          location
          websiteUrl
          twitterUsername
          contributionsCollection {
            contributionCalendar {
              weeks {
                contributionDays {
                  date
                  contributionCount
                }
              }
            }
          }
        }
      }
    "#;

    // Build a JSON map of variables:
    let mut vars = HashMap::new();
    vars.insert("login".to_string(), json!(&args.username));

    // Wrap entire GraphQL request in a serde_json::Value
    let payload = json!({
      "query": query,
      "variables": vars
    });

    // Deserialize the body into a serde_json::Value
    let resp_value: serde_json::Value = gh
        .post::<serde_json::Value, _>("https://api.github.com/graphql", Some(&payload))
        .await?;

    // Deserialize into your strongly‐typed model:
    let resp: GraphqlResponse =
        serde_json::from_value(resp_value).expect("failed to deserialize GraphQL response");

    let user = resp.data.user;

    // Fetch the raw image bytes with reqwest
    let img_bytes = reqwest::get(user.avatar_url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    // Decode into an image::DynamicImage
    let img = ImageReader::new(std::io::Cursor::new(img_bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    let config = Config {
        // position on the screen:
        x: 0,
        y: 0,
        absolute_offset: false,
        // resize to fit width/height:
        width: Some(20),
        height: Some(10),
        // other defaults:
        ..Default::default()
    };

    let mut days: Vec<(String, u32)> = Vec::new();
    for week in user.contributions_collection.contribution_calendar.weeks {
        for day in week.contribution_days {
            days.push((day.date, day.contribution_count));
        }
    }
    let counts: Vec<u32> = days.iter().map(|(_, c)| *c).collect();

    let chart = sparkline_log(&counts);

    /* Section, where the output is printed */

    viuer::print(&img, &config).unwrap();
    print!("Github:\t{}", user.login);
    if let Some(name) = &user.name {
        print!("\tName:\t{}", name);
    }
    // if let Some(email) = &user.email {
    //     print!("\tEmail:\t{}", email);
    // }
    // if let Some(company) = &user.company {
    //     print!("\tCompany:\t{}", company);
    // }
    // if let Some(bio) = &user.bio {
    //     print!("\tBio:\n{}", bio);
    // }
    //
    println!();
    println!("Contribution Chart over the last year:");
    println!("{}", chart);

    Ok(())
}
