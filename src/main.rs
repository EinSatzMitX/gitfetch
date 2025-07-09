#![allow(dead_code)]
use octocrab::{Octocrab, models::SimpleUser};

use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, env};

#[derive(Deserialize, Debug)]
struct ContribDay {
    date: String,
    contrib_count: u32,
}

#[derive(Deserialize, Debug)]
struct Week {
    contrib_days: Vec<ContribDay>,
}

#[derive(Deserialize, Debug)]
struct ContributionCalendar {
    weeks: Vec<Week>,
}

#[derive(Deserialize, Debug)]
struct ContributionsCollection {
    contribution_calendar: ContributionCalendar,
}

#[derive(Deserialize, Debug)]
struct UserData {
    contributions_collection: ContributionsCollection,
}

#[derive(Deserialize, Debug)]
struct GraphqlData {
    user: UserData,
}

struct CliArgs {
    token: Option<String>,
}

fn print_usage() {
    println!("Usage: gitfetch API_TOKEN");
}

fn parse_args() -> CliArgs {
    // skip the first arg (its the program name)
    let args: Vec<String> = std::env::args().skip(1).collect();
    // if args.is_empty() {
    //     print_usage();
    //     std::process::exit(-1);
    // }

    CliArgs {
        token: match args.len() {
            0 => None,
            _ => Some(args[0].clone()),
        },
    }
}

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    let args = parse_args();
    let gh;
    if let Some(tok) = args.token {
        println!("got a token with length: {}", tok.len());
        gh = Octocrab::builder().personal_token(tok).build()?;
    } else {
        println!("No token provided, you may be rate limited!");
        gh = Octocrab::builder().build()?;
    }
    let query = r#"
      query($login: String!) {
        user(login: $login) {
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
    vars.insert("login".to_string(), json!("EinSatzMitX"));

    // Wrap entire GraphQL request in a serde_json::Value
    let payload = json!({
      "query": query,
      "variables": vars
    });

    let response = gh
        ._post("https://api.github.com/graphql", Some(&payload))
        .await?; // response: http::Response<…>

    // 2. Deserialize the body into a serde_json::Value
    let resp_value: serde_json::Value = gh
        .post::<serde_json::Value, _>("https://api.github.com/graphql", Some(&payload))
        .await?;

    println!("{:#}", resp_value);

    Ok(())
}
