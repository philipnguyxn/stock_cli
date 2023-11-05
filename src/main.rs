use chrono::offset::Local;
use chrono::{DateTime, Datelike, Duration, TimeZone};
use core::panic;
use dotenv::dotenv;
use plotters::prelude::{BitMapBackend, CandleStick, ChartBuilder, IntoDrawingArea};
use plotters::style::{Color, IntoFont, GREEN, RED, WHITE};
use std::env;
use std::fs::create_dir_all;
use std::path::Path;

use exitfailure::ExitFailure;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct StockCandles {
    c: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    o: Vec<f64>,
    s: String,
    t: Vec<i64>,
    v: Vec<i128>,
}

impl StockCandles {
    async fn get(
        symbol: &String,
        from_date: DateTime<Local>,
        to_date: DateTime<Local>,
    ) -> Result<Self, ExitFailure> {
        dotenv().ok();

        let finnhub_api_key: String = env::var("FINNHUB_API_KEY")
            .expect("Error: Finnhub's api key not found.")
            .to_string();

        let url =
            format!(
            "https://finnhub.io/api/v1/stock/candle?symbol={}&resolution=W&from={}&to={}&token={}",
            symbol, from_date.timestamp(), to_date.timestamp(), finnhub_api_key
        );

        println!(
            "Fetching {}'s price data from {} to {}",
            symbol,
            from_date.format("%d-%m-%Y").to_string(),
            to_date.format("%d-%m-%Y").to_string()
        );

        let response = reqwest::get(&url).await?.json::<StockCandles>().await?;

        if response.s != "ok" {
            panic!("Error: {}", response.s);
        }

        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    let args: Vec<String> = env::args().collect();
    let mut symbol: String = "AAPL".to_string();

    if args.len() < 2 {
        println!("No symbol provided, using default: {}", symbol);
    } else {
        symbol = args[1].clone();
    }

    // Fetch stock candles
    let (from_date, to_date) = (Local::now() - Duration::days(365), Local::now());

    let stock_candles = StockCandles::get(&symbol, from_date, to_date).await?;

    println!("{}'s price data fetched successfully", &symbol);
    println!("Plotting {}'s price data", &symbol);

    // Collect the data in the stock_candles struct into individual vectors
    let close_prices: Vec<f64> = stock_candles.c.iter().map(|&c| c).collect();
    let open_prices: Vec<f64> = stock_candles.o.iter().map(|&o| o).collect();
    let high_prices: Vec<f64> = stock_candles.h.iter().map(|&h| h).collect();
    let low_prices: Vec<f64> = stock_candles.l.iter().map(|&l| l).collect();
    let timestamps: Vec<DateTime<Local>> = stock_candles
        .t
        .iter()
        .filter_map(|&t| parse_time(t).ok())
        .collect();

    // Filter the timestamps to only include the first day of each month
    let mut labels_iter = timestamps.iter().peekable();
    let mut labels: Vec<DateTime<Local>> = Vec::new();

    while let Some(current_date) = labels_iter.next() {
        let next_date = labels_iter.peek().map(|&date| date);
        if should_show_label(current_date, next_date) {
            labels.push(current_date.clone());
        }
    }

    let out_file_name = format!("./static/{}.png", &symbol);
    create_directory("./static")?;

    let root = BitMapBackend::new(out_file_name.as_str(), (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    // Create the chart
    let highest_price: f64 = high_prices.clone().into_iter().reduce(f64::max).unwrap() + 25.0;
    let lowest_price: f64 = low_prices.clone().into_iter().reduce(f64::min).unwrap() - 25.0;

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("{} Stock Price", &symbol),
            ("sans-serif", 50).into_font(),
        )
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(from_date..to_date, lowest_price..highest_price)?;

    // Configure the mesh axes
    chart
        .configure_mesh()
        .light_line_style(WHITE)
        .x_labels(labels.len())
        .x_label_formatter(&|timestamp| timestamp.format("%d-%m-%Y").to_string())
        .draw()?;

    // Plot the candle sticks
    close_prices
        .iter()
        .enumerate()
        .for_each(|(index, &close_price)| {
            let open_price = open_prices[index];
            let high_price = high_prices[index];
            let low_price = low_prices[index];
            let timestamp = timestamps[index];

            // Create the CandleStick instance
            let candle_stick = CandleStick::new(
                timestamp,
                open_price,
                high_price,
                low_price,
                close_price,
                GREEN.filled(),
                RED.filled(),
                15, // Width of the candle stick
            );

            // Draw the CandleStick on the chart
            // You would handle the Result here or use `expect`/`unwrap` if you are sure it should never fail
            chart
                .draw_series(std::iter::once(candle_stick))
                .expect("Failed to draw series");
        });

    root.present().expect(
        "Unable to write result to file, please make sure 'static' dir exists under current dir",
    );
    println!("Result has been saved to {}", out_file_name);

    Ok(())
}

fn parse_time(timestamp: i64) -> Result<DateTime<Local>, ExitFailure> {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .ok_or_else(|| ExitFailure::from(failure::err_msg("Failed to parse timestamp")))
}

fn create_directory(dir_name: &str) -> Result<(), ExitFailure> {
    let path = Path::new(dir_name);

    if let Err(e) = create_dir_all(path) {
        panic!("Error: {:?}", e);
    } else {
        println!("Directory created successfully");
        Ok(())
    }
}

fn should_show_label(current_date: &DateTime<Local>, next_date: Option<&DateTime<Local>>) -> bool {
    match next_date {
        Some(next_date) => {
            current_date.month() != next_date.month() || current_date.year() != next_date.year()
        }
        None => true,
    }
}
