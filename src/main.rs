use std::{error::Error, thread::sleep, process::Command, collections::HashMap};
use thirtyfour::prelude::*;
use std::env;
use tokio;
// 528ffzlg9@mozmail.com
// Cw;-)X42&+w9MD!
struct Settings {
    email: String,
    password: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct LanguageTranslation {
    words: Vec<String>,
    language: String,
}

struct TranslationDictionary {
    translations: HashMap<LanguageTranslation, LanguageTranslation>,
}

impl TranslationDictionary {
    fn new() -> Self {
        TranslationDictionary {
            translations: HashMap::new(),
        }
    }

    /// this is going to translate both words and sentences
    /// where if the word/sentence is not in the dictionary
    /// it calles the google translate api and adds it to 
    /// the dictionary
    /// 
    /// if the word/sentence is in the dictionary
    /// it will be returned
    
    fn translate(&mut self, words: &Vec<String>, from_language: String, to_language: String) -> String {
        todo!("find a good way to structure this")
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env::set_var("WEBDRIVER_GECKO_DRIVER", "C:/Users/noahb/Documents/programs/geckodriver.exe");

    let _driver_process = Command::new("C:/Users/noahb/Documents/programs/geckodriver.exe")
        .spawn()
        .expect("Failed to start geckodriver");
    
    let settings = Settings {
        email: "528ffzlg9@mozmail.com".to_string(),
        password: "Cw;-)X42&+w9MD!".to_string(),
    };

    let caps = DesiredCapabilities::firefox();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;
    login(&driver, settings.email, settings.password).await?;
    while driver.current_url().await?.as_str() != "https://www.duolingo.com/learn" {
        sleep(std::time::Duration::from_millis(100));
    }
    sleep(std::time::Duration::from_secs(1));
    
    let translation_dictionary = TranslationDictionary::new();
    let lessons = driver.find_all(By::XPath("//div[@class='_31n11 _3DQs0']")).await?;
    println!("Found {} lessons", lessons.len());
    for lesson in lessons {
        lesson.click().await?;
        driver.find(By::XPath("//a[@class='_30qMV _2N_A5 _36Vd3 _16r-S KSXIb _2CJe1 _12StQ']")).await?.click().await?;
        while driver.current_url().await?.as_str() != "https://www.duolingo.com/lesson" {
            sleep(std::time::Duration::from_millis(100));
        }
        println!("Entered lesson");
        solve_challanges(&driver, &translation_dictionary).await?;
    }
    Ok(())
}

#[derive(Debug)]
enum ChallangeTypes {
    Select,
    Translate,
}

async fn get_challange_type(driver: &WebDriver) -> Result<ChallangeTypes, Box<dyn Error + Send + Sync>> {
    let challange_type_attr: String = driver.find(By::XPath("//div[@class='e4VJZ FQpeZ']"))
        .await?
        .attr("data-test")
        .await?
        .unwrap();

    let challange_type_attr_cleaned: Vec<&str> = challange_type_attr
        .as_str()
        .split(" ")
        .collect();

    let challange_type = match challange_type_attr_cleaned[1] {
        "challenge-translate" => ChallangeTypes::Translate,
        "challenge-select" => ChallangeTypes::Select,
        _ => panic!("Unknown challange type: {}", challange_type_attr_cleaned[1]),
    };
    println!("Challange type: {:?}", challange_type);
    println!("Challange text: {}", challange_type_attr_cleaned[1]);
    Ok(challange_type)
}

async fn solve_challanges(driver: &WebDriver, translation_dictionary: &TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    match get_challange_type(driver).await? {
        ChallangeTypes::Select => solve_select_challange(driver, translation_dictionary).await?,
        ChallangeTypes::Translate => solve_translate_challange(driver, translation_dictionary).await?,
    }
    Ok(())
}

async fn solve_select_challange(driver: &WebDriver, translation_dictionary: &TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    let available_choices: Vec<WebElement> = driver.find_all(By::XPath("//div[@data-test='challenge-choice']"))
        .await?;

    Ok(())
}

async fn solve_translate_challange(driver: &WebDriver, translation_dictionary: &TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    todo!("solve translate challange")
}

async fn login(driver: &WebDriver, email: String, password: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    driver.goto("https://www.duolingo.com/").await?;
    driver.find(By::XPath("//button[@data-test='have-account']")).await?.click().await?;
    driver.find(By::XPath("//input[@data-test='email-input']"))
        .await?
        .send_keys(email)
        .await?;
    driver.find(By::XPath("//input[@data-test='password-input']"))
        .await?
        .send_keys(password)
        .await?;
    driver.find(By::XPath("//button[@data-test='register-button']"))
        .await?
        .click()
        .await?;
    Ok(())
}
    