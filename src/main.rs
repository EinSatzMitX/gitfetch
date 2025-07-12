// #![allow(dead_code)]
use image::ImageReader;
use octocrab::Octocrab;

use clap::{ArgAction, Parser};
use serde::Deserialize;
use serde_json::{from_str, json};
use std::{
    collections::HashMap, env::home_dir, fs::read_to_string, path::PathBuf, thread::current,
};
use viuer::Config;

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

/* Sparkline representation, but it scales logarithmically */
fn sparkline_log(data: &[u32], color_levels: Option<Vec<(u8, u8, u8)>>) -> String {
    // Unicode blocks for height
    const BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

    // GitHub contribution colors (hex → RGB)
    const DEFAULT_COLORS: [(u8, u8, u8); 5] = [
        (33, 110, 57),   // level 4: darker green
        (48, 161, 78),   // level 3: dark green
        (64, 196, 99),   // level 2: medium green
        (155, 233, 168), // level 1: light green
        (235, 237, 240), // level 0: very light gray (no contributions)
    ];

    let colors = match color_levels {
        Some(ref custom) if custom.len() >= 5 => custom.clone(),
        _ => DEFAULT_COLORS.to_vec(),
    };

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
            let (r, g, b) = colors[lvl];
            // ANSI 24‐bit color: set fg to (r,g,b), print block, reset
            format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, BLOCKS[h])
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct GitfetchConfig {
    color_levels: Option<Vec<(u8, u8, u8)>>,
    username_color: Option<(u8, u8, u8)>,
    string_modules: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct StringModule {
    contents: String,
    // The name, that will be used in the json config
    name: String,
}

fn load_config() -> Option<GitfetchConfig> {
    let mut path = PathBuf::new();

    if let Some(home) = home_dir() {
        path.push(home);
        path.push(".config/gitfetch/config.json");
    } else {
        eprintln!("Could not determine home directory!");
        return None;
    }

    if !path.exists() {
        eprintln!("No config file found at {:?}", path);
        return None;
    }

    let contents = read_to_string(&path).ok().unwrap();
    let config: GitfetchConfig = from_str(&contents)
        .ok()
        .expect("JSON contents can't be read! (Did you write the json file by hand?)");
    Some(config)
}

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    let args = CliArgs::parse();
    let mut gitfetch_config = load_config();
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

    let viuer_config = Config {
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
    let mut total_contribs: u32 = 0;

    let mut days: Vec<(String, u32)> = Vec::new();
    for week in user.contributions_collection.contribution_calendar.weeks {
        for day in week.contribution_days {
            days.push((day.date, day.contribution_count));
            total_contribs += day.contribution_count;
        }
    }
    let counts: Vec<u32> = days.iter().map(|(_, c)| *c).collect();

    let color_levels = gitfetch_config
        .as_ref()
        .and_then(|cfg| cfg.color_levels.clone());
    let chart = sparkline_log(&counts, color_levels);
    let chart_module = StringModule {
        contents: chart,
        name: "chart_module".to_string(),
    };

    let github_name_colors = gitfetch_config
        .as_ref()
        .and_then(|cfg| cfg.username_color.clone())
        .unwrap();

    let unique_name_module = StringModule {
        contents: format!(
            "\x1b[38;2;{};{};{}mGithub:\t{}\x1b[0m",
            github_name_colors.0, github_name_colors.1, github_name_colors.2, user.login
        ),
        name: "unique_name_module".to_string(),
    };

    let custom_name_module = StringModule {
        contents: match &user.name {
            Some(name) => {
                format!("Display name: {}", name)
            }
            None => {
                format!("")
            }
        },
        name: "custom_name_module".to_string(),
    };

    let total_contribs_fmt_module = StringModule {
        contents: format!("Total Contributions over the last year: {}", total_contribs),
        name: "total_contribs_fmt_module".to_string(),
    };

    let default_modules: Vec<StringModule> = vec![
        unique_name_module,
        custom_name_module,
        total_contribs_fmt_module,
        chart_module,
        // …add new modules here in the desired default order…
    ];

    // 2) Also build a lookup map so we can grab modules by name fast
    let mut module_map: HashMap<_, _> = default_modules
        .iter()
        .map(|m| (m.name.clone(), m.clone()))
        .collect();

    // 3) Decide final order
    let modules_to_render: Vec<StringModule> = if let Some(cfg) = &gitfetch_config {
        if let Some(order) = &cfg.string_modules {
            // User-specified order: keep only known names, in that sequence
            order
                .iter()
                .filter_map(|name| module_map.remove(name))
                .collect()
        } else {
            // No user order ⇒ use your default Vec
            default_modules.clone()
        }
    } else {
        // No config at all ⇒ default
        default_modules.clone()
    };

    /* Section, where the output is printed */

    viuer::print(&img, &viuer_config).unwrap();
    for i in modules_to_render {
        println!("{}", i.contents);
    }

    Ok(())
}
