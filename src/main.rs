use chrono::offset::Local;
use chrono::{DateTime, Duration};
use dotenv::dotenv;
use plotters::prelude::{BitMapBackend, CandleStick, ChartBuilder, IntoDrawingArea};
use plotters::style::{Color, IntoFont, GREEN, RED, WHITE};
use std::env;

use exitfailure::ExitFailure;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct StockCandles {
    c: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    o: Vec<f64>,
    s: Vec<String>,
    t: Vec<i128>,
    v: Vec<i128>,
}

impl StockCandles {
    async fn get(
        symbol: &String,
        from_date: &DateTime<Local>,
        to_date: &DateTime<Local>,
    ) -> Result<Self, ExitFailure> {
        dotenv().ok();

        let finnhub_api_key: String = env::var("FINNHUB_API_KEY")
            .expect("Error: Finnhub's api key not found.")
            .to_string();

        let url =
            format!(
            "https://finnhub.io/api/v1/stock/candle?symbol={}&resolution=D&token={}&from={}&to={}",
            symbol, finnhub_api_key, from_date.timestamp(), to_date.timestamp()
        );
        let response = reqwest::get(&url).await?.json::<StockCandles>().await?;

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

    let stock_candles = StockCandles::get(&symbol, &from_date, &to_date).await?;

    // Plotting
    let out_file_name = format!("./static/{}.png", symbol).as_str();

    let root = BitMapBackend::new(out_file_name, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("{} Stock Price", symbol),
            ("sans-serif", 50).into_font(),
        )
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(from_date..to_date, 0f64..200f64)?;

    chart.configure_mesh().light_line_style(WHITE).draw()?;

    chart.draw_series(
        stock_candles
            .c
            .iter()
            .zip(stock_candles.t.iter())
            .map(|(y, x)| CandleStick::new(x, open, high, low, close, GREEN.filled(), RED, 15)),
    )?;

    root.present().expect(
        "Unable to write result to file, please make sure 'static' dir exists under current dir",
    );
    println!("Result has been saved to {}", out_file_name);

    Ok(())
}