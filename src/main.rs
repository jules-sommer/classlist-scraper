use reqwest::Error;
use std::{fmt::Display, thread::sleep};
use thirtyfour::{
  common::capabilities::firefox::LogLevel,
  error::WebDriverError,
  extensions::query::{ElementQueryable, ElementWaitable},
  By, DesiredCapabilities, WebDriver,
};

// import std base64
use base64::prelude::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
  static ref TARGET_URL: &'static str = "https://bcs.edsby.com/p/BasePublic";
  static ref TARGET_SCHOOL_URL: &'static str =
    "https://bcs.edsby.com/p/District/";
}

trait ToSnakeCase {
  fn to_snake_case(&self) -> String;
}

impl ToSnakeCase for str {
  fn to_snake_case(&self) -> String {
    let mut result = String::with_capacity(self.len());
    for (i, c) in self.chars().enumerate() {
      if c.is_uppercase() {
        if i > 0 {
          result.push('_');
        }
        // Directly convert uppercase ASCII characters to lowercase
        if c.is_ascii() {
          result.push(((c as u8) + 32) as char);
        } else {
          // Fallback for non-ASCII characters
          result.extend(c.to_lowercase());
        }
      } else {
        result.push(c);
      }
    }
    result
  }
}

#[derive(
  Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Page {
  url: String,
  title: String,
  source: String,
  screenshot: Option<Vec<u8>>,
}

impl Display for Page {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // serialize the page to a JSON string
    let json = serde_json::to_string(self).unwrap();
    write!(f, "{}", json)
  }
}

pub trait Base64Encoded {
  fn to_base64(&self) -> String;
  fn from_base64(b64_str: &str) -> Result<Self, base64::DecodeError>
  where
    Self: Sized;
}

impl Base64Encoded for Vec<u8> {
  fn to_base64(&self) -> String {
    BASE64_STANDARD.encode(&self)
  }
  fn from_base64(b64_str: &str) -> Result<Self, base64::DecodeError> {
    BASE64_STANDARD.decode(b64_str)
  }
}

#[derive(Debug, Clone)]
pub struct Browser {
  driver: WebDriver,
  // body: Option<thirtyfour::WebElement>,
  // cookies: Vec<thirtyfour::Cookie>,
}

impl Browser {
  // Asynchronously create a new instance of the browser
  pub async fn new(port: i16) -> Result<Self, WebDriverError> {
    let mut caps = DesiredCapabilities::firefox();
    // caps.set_headless()?;
    caps.set_log_level(LogLevel::Trace)?;
    let driver_result =
      WebDriver::new(format!("http://localhost:{:?}", port), caps).await;

    match driver_result {
      Ok(driver) => Ok(Self { driver }),
      Err(e) => Err(e),
    }
  }

  pub async fn screenshot(&self) -> Result<Vec<u8>, WebDriverError> {
    self.driver.screenshot_as_png().await
  }

  // Navigate to a URL
  pub async fn navigate(
    &self,
    url: &str,
  ) -> Result<Page, Box<dyn std::error::Error>> {
    self.driver.get(url).await?;
    // wait for the page to load
    let body = self.driver.query(By::Tag("body")).first().await?;
    body
      .wait_until()
      .has_attribute("data-runtime-theme", "default")
      .await?;

    let title = self.driver.title().await?;

    let session_cookie =
      self.driver.get_named_cookie("session_id_edsby").await?;

    println!("Getting session cookie...");

    if session_cookie.value.len() <= 0 {
      println!("No session cookie found");
    } else {
      println!("SESSION COOKIE: {:?}", session_cookie);
    }

    if title.contains("Login") {
      // find login form
      let login_form = self.driver.query(By::Id("3loginform")).first().await?;

      let inputs = login_form.find_all(By::Tag("input")).await?;
      for input in inputs {
        let name = input.attr("name").await?;
        match name {
          Some(name) => {
            println!("Input: {:?}", name.clone());
            match name.as_str() {
              "login-userid" => {
                input.send_keys("rsommer").await?;
              }
              "login-password" => {
                input.send_keys("BCS2022!rs").await?;
              }
              "remember" => {
                println!("{:?}", input.to_json());
              }
              _ => {
                println!("{:?}", input.to_json());
              }
            }
          }
          None => {
            println!("No name attribute found");
            println!("Input: {:?}", input.to_json());
          }
        }
      }

      let login_button = body.find(By::Id("3loginform-login-submit")).await?;
      login_button.wait_until().enabled().await?;

      let screenshot_loginbtn = login_button.screenshot_as_png().await?;
      std::fs::write("login_button.png", screenshot_loginbtn).unwrap();

      login_button.click().await?;

      let session_cookie =
        self.driver.get_named_cookie("session_id_edsby").await?;
      if session_cookie.value.len() <= 0 {
        println!("No session cookie found");
      } else {
        println!("SESSION COOKIE: {:?}", session_cookie);
      }
    }

    let source = self.driver.source().await?;
    let screenshot = self.driver.screenshot_as_png().await?;

    Ok(Page {
      url: url.to_string(),
      title,
      source,
      screenshot: Some(screenshot),
    })
  }
  pub async fn close(self) -> Result<(), WebDriverError> {
    self.driver.quit().await
  }
}

fn get_school_url(school_id: i32) -> String {
  format!("{}{}", *TARGET_SCHOOL_URL, school_id)
}

#[tokio::main]
async fn main() {
  let start_headless_res = Browser::new(4444).await;
  let browser = match start_headless_res {
    Ok(browser) => browser,
    Err(e) => {
      println!("Error: {:?}", e);
      return;
    }
  };
  let school_url = get_school_url(21471167);

  let page = browser.navigate(&school_url).await.unwrap();
  let title = page.title.clone().to_snake_case();
  println!("Page: {}", title);
  // save screenshot to a file
  std::fs::write(
    format!("{:?}.png", title.clone()),
    page.clone().screenshot.unwrap(),
  )
  .unwrap();

  // serialize the page to a JSON string
  std::fs::write(format!("{:?}.json", title.clone()), page.to_string())
    .unwrap();

  std::fs::write(format!("{:?}.html", title.clone()), page.source).unwrap();

  sleep(std::time::Duration::from_secs(5));

  let close_res = browser.close().await;
  match close_res {
    Ok(_) => println!("Browser closed"),
    Err(e) => println!("Error: {:?}", e),
  }
}
