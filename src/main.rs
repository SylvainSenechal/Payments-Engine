use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct Transaction {
    #[serde(rename = "type")]
    category: TransactionCategory,
    #[serde(rename = "client")]
    client_id: u16,
    tx: u32,
    amount: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum TransactionCategory {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
struct Client {
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

impl Default for Client {
    fn default() -> Self {
        Client {
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }
}

// https://docs.rs/csv/1.1.6/csv/tutorial/index.html
fn main() -> std::io::Result<()> {
    let transactions = read_from_file("src/testSamples/input1.csv")?;
    let mut clients: HashMap<u16, Client> = HashMap::new();

    println!("{:?}", transactions);
    println!("{:?}", clients);
    process_transactions(&transactions, &mut clients);
    println!("{:?}", clients);
    Ok(())
}

fn read_from_file(file_path: &str) -> Result<Vec<Transaction>, csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file_path)?;

    rdr.deserialize()
        .collect::<Result<Vec<Transaction>, csv::Error>>()
}

fn process_transactions(transactions: &Vec<Transaction>, clients: &mut HashMap<u16, Client>) {
    for t in transactions {
        let client = clients.entry(t.client_id).or_default();

        match t.category {
            TransactionCategory::Deposit => deposit(t.amount, client),
            TransactionCategory::Withdrawal => withdraw(t.amount, client),
            _ => (),
        }
    }
}

fn deposit(amount: f32, client: &mut Client) {
    client.available += amount;
    client.total += amount
}

fn withdraw(amount: f32, client: &mut Client) {
    if amount < client.available {
        client.available -= amount
    }
}
