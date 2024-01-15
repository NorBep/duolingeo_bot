use std::collections::HashMap;
use std::thread::sleep;
use std::error::Error;
use std::sync::Arc;

use serde::Deserialize;
use thirtyfour::prelude::*;
use lingual::{Lang, blocking::translate};
use futures;
use regex;
use tokio;
// 528ffzlg9@mozmail.com
// Cw;-)X42&+w9MD!

#[derive(Debug, PartialEq, Eq, Hash)]
struct LanguageTranslation {
    text: String,
    from_language: Lang,
    to_language: Lang,
}

#[allow(unused)]
struct TranslationDictionary {
    translations: HashMap<LanguageTranslation, Vec<String>>,
}

#[allow(unused)]
impl TranslationDictionary {
    fn new() -> Self {
        TranslationDictionary {
            translations: HashMap::new(),
        }
    }

    /// this is going to translate both words and sentences
    /// where if the word/sentence is not in the dictionary
    /// it calls the google translate api and adds it to 
    /// the dictionary
    /// 
    /// if the word/sentence is in the dictionary
    /// it will be returned
    
    fn translate(&mut self, text: &[String], from_language: Lang, to_language: Lang) -> String {    
        // generate the key for the hash dictionary
        let language_translation = if text.len() == 1 {
            LanguageTranslation {
                text: text[0].clone(),
                from_language,
                to_language,
            }
        } else {
            let sentence = text.join(" ");
            LanguageTranslation {
                text: sentence,
                from_language,
                to_language,
            }
        };
        
        match self.translations.get(&language_translation) {
            Some(translations) => {
                // find out if the translation is a word or a sentence
                // if it is a word return the translation
                // if it is a sentence return the translation with the words in the same order
                if text.len() == 1 {
                    translations[0].clone()
                } else {
                    translations.join(" ")
                }    
            },
            None => {
                // translate the word
                // add the translation to the dictionary
                // return the translation
                let translated_word = translate(language_translation.text.clone(), Some(Lang::En), Some(Lang::Nl)).unwrap().text().to_string();
                self.translations.insert(language_translation, vec![translated_word.clone()]);
                translated_word
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_dictionary() {
        let mut translation_dictionary = TranslationDictionary::new();

        let words = vec!["Have a good night sleep and goodnight".to_string()];
        let from_language = Lang::En;
        let to_language = Lang::Nl;
        
        let start = std::time::Instant::now();
        let translation = translation_dictionary.translate(&words, from_language, to_language);
        let end = std::time::Instant::now();
        println!("Time to translate: {:?}", end - start);
        assert_eq!(translation, "Heb een goede nachtrust en welterusten");

        // now the translation should be in the dictionary and should take less time
        let start = std::time::Instant::now();
        let translation = translation_dictionary.translate(&words, from_language, to_language);
        let end = std::time::Instant::now();
        println!("Time to translate: {:?}", end - start);
        assert_eq!(translation, "Heb een goede nachtrust en welterusten");
    }

    #[test]
    fn test_load_settings() {
        let settings = load_settings(std::path::Path::new("./settings.json"));
        assert_eq!(settings.headless, false);
        assert_eq!(settings.password, "Cw;-)X42&+w9MD!");
    }
}

#[derive(Debug, Deserialize)]
struct Settings {
    pub headless: bool,
    pub email: String,
    pub password: String,
}

fn load_settings(fp: &std::path::Path) -> Settings {
    let settings = std::fs::read_to_string(fp).unwrap();
    let settings: Settings = serde_json::from_str(&settings).unwrap();
    settings
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    std::env::set_var("WEBDRIVER_GECKO_DRIVER", "C:/Users/noahb/Documents/programs/geckodriver.exe");

    let _driver_process = std::process::Command::new("C:/Users/noahb/Documents/programs/geckodriver.exe")
        .spawn()
        .expect("Failed to start geckodriver");
    
    let settings = load_settings(std::path::Path::new("./settings.json"));
    let mut caps = DesiredCapabilities::firefox();
    if settings.headless {
        caps.add_firefox_option("args", vec!["-headless"])?;
    }
    let driver = WebDriver::new("http://localhost:4444", caps).await?;
    login(&driver, settings.email, settings.password).await?;
    while driver.current_url().await?.as_str() != "https://www.duolingo.com/learn" {
        sleep(std::time::Duration::from_millis(100));
    }
    sleep(std::time::Duration::from_secs(1));
    let translation_dictionary = Arc::new(tokio::sync::Mutex::new(TranslationDictionary::new()));
    let lessons = driver.find_all(By::XPath("//div[@class='_31n11 _3DQs0']")).await?;

    let mut tasks = vec![];

    for lesson in lessons {
        let driver = driver.clone();
        let translation_dictionary = Arc::clone(&translation_dictionary);
        let task = tokio::spawn(async move {
            lesson.click().await?;
            driver.find(By::XPath("//a[@class='_30qMV _2N_A5 _36Vd3 _16r-S KSXIb _2CJe1 _12StQ']")).await?.click().await?;
            println!("Entered lesson");
            let mut translation_dictionary = translation_dictionary.lock().await;
            solve_challanges(&driver, &mut *translation_dictionary).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        tasks.push(task);
    }
    for task in tasks {
        task.await??;
    }
    Ok(())
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

#[derive(Debug)]
enum ChallangeTypes {
    Select,
    Translate,
}

async fn solve_challanges(driver: &WebDriver, translation_dictionary: &mut TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    sleep(std::time::Duration::from_secs(1));
    match get_challange_type(driver).await? {
        ChallangeTypes::Select => solve_select_challange(driver, translation_dictionary).await?,
        ChallangeTypes::Translate => solve_translate_challange(driver, translation_dictionary).await?,
    }
    Ok(())
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
    Ok(challange_type)
}

async fn solve_select_challange(driver: &WebDriver, translation_dictionary: &mut TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    let challange_header = driver.find(By::XPath("//h1[@data-test='challenge-header']"))
        .await?
        .text()
        .await?;
    let regex = regex::Regex::new(r#"(?m)“([^”]*)”"#).unwrap();
    let challange_text = regex.captures(&challange_header).unwrap()[1].to_string();

    let available_choices: Vec<WebElement> = driver.find_all(By::XPath("//div[@data-test='challenge-choice']/span[1]")).await?;
    let choice_texts: Vec<_> = available_choices.iter()
        .map(|choice| choice.text())
        .collect();
    let choices_results: Vec<Result<String, WebDriverError>> = futures::future::join_all(choice_texts).await;    
    let choices: Vec<String> = choices_results.into_iter().map(|res| res.unwrap()).collect();

    let translated_challange_text = translation_dictionary.translate(&[challange_text], Lang::En, Lang::Nl);
    let translated_choices = choices.iter()
        .map(|choice| translation_dictionary.translate(&[choice.clone()], Lang::En, Lang::Nl))
        .collect::<Vec<_>>();
    let mut choice_index = 0;
    for (index, choice) in translated_choices.iter().enumerate() {
        if choice == &translated_challange_text {
            choice_index = index;
            break;
        }
    }
    println!("chosen: {}", available_choices[choice_index].text().await?);
    available_choices[choice_index].click().await?;
    driver.find(By::XPath("//button[@data-test='player-next']"))
        .await?
        .click()
        .await?;
    Ok(())
}

#[allow(unused)]
async fn solve_translate_challange(driver: &WebDriver, translation_dictionary: &TranslationDictionary) -> Result<(), Box<dyn Error + Send + Sync>> {
    todo!("solve translate challange")
}

#[test]
fn test_regex() {
    let regex = regex::Regex::new(r#"(?m)“([^”]*)”"#).unwrap();
    let text = r#"Which one of these is “the girl”?"#;
    
    // hack incomming
    let capture = regex.captures(text).unwrap();
    println!("{:?}", &capture[1]);
}