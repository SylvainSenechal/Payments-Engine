use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Deserialize, Clone, Debug)]
struct Transaction {
    #[serde(rename = "type")]
    category: TransactionCategory,
    #[serde(rename = "client")]
    client_id: u16,
    tx: u32,
    amount: Option<f32>,
}

#[derive(Deserialize, Clone, Debug)]
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
// 4 digit precision
// map error
// handle empty column csv, or wrong type
// check tx or client id out of u16/u32
// handle double dispute / double resolve
// https://docs.rs/csv/1.1.6/csv/tutorial/index.html
fn main() -> std::io::Result<()> {
    let transactions = read_from_file("src/testSamples/input4.csv")?;
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
    let mut transactions_history: HashMap<u32, Transaction> = HashMap::new();
    let mut ongoing_disputes: HashSet<u32> = HashSet::new();
    for t in transactions {
        // Get client of the transaction + initialize if it doesn't exists
        let client = clients.entry(t.client_id).or_default();

        if !client.locked {
            match t.category {
                TransactionCategory::Deposit => deposit(t.amount.unwrap(), client),
                TransactionCategory::Withdrawal => withdraw(t.amount.unwrap(), client),
                TransactionCategory::Dispute => {
                    dispute(t.tx, &transactions_history, &mut ongoing_disputes, client)
                }
                TransactionCategory::Resolve => {
                    resolve(t.tx, &transactions_history, &mut ongoing_disputes, client)
                }
                TransactionCategory::Chargeback => {
                    charge_back(t.tx, &transactions_history, &mut ongoing_disputes, client)
                }
            }
            if let TransactionCategory::Deposit = t.category {
                transactions_history.insert(t.tx, t.to_owned());
            }
        }

        println!(" history {:?}", transactions_history);
    }
}

fn deposit(amount: f32, client: &mut Client) {
    client.available += amount;
    client.total += amount;
}

fn withdraw(amount: f32, client: &mut Client) {
    if amount < client.available {
        client.available -= amount;
        client.total -= amount;
    }
}

fn dispute(
    transaction_disputed_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    if let Some(disputed) = transactions_history.get(&transaction_disputed_id) {
        // check ongoing dispute first ?
        match disputed.category {
            TransactionCategory::Deposit => {
                // Can't dispute twice the same transaction
                if !ongoing_disputes.contains(&disputed.tx) {
                    client.available -= disputed.amount.unwrap();
                    client.held += disputed.amount.unwrap();
                    ongoing_disputes.insert(disputed.tx);
                }
            }
            _ => (), // Not sure how to handle dispute on the other kind of transactions
        }
    }
}

fn resolve(
    transaction_resolved_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    if let Some(resolved) = transactions_history.get(&transaction_resolved_id) {
        match resolved.category {
            TransactionCategory::Deposit => {
                // Can't resolve a transaction that isn't under dispute
                if ongoing_disputes.contains(&resolved.tx) {
                    client.available += resolved.amount.unwrap();
                    client.held -= resolved.amount.unwrap();
                    ongoing_disputes.remove(&resolved.tx);
                }
            }
            _ => (),
        }
    }
}

fn charge_back(
    transaction_charged_back_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    if let Some(charged_back) = transactions_history.get(&transaction_charged_back_id) {
        match charged_back.category {
            TransactionCategory::Deposit => {
                if ongoing_disputes.contains(&charged_back.tx) {
                    client.held -= charged_back.amount.unwrap();
                    client.total -= charged_back.amount.unwrap();
                    client.locked = true;
                    ongoing_disputes.remove(&charged_back.tx);
                }
            }
            _ => (),
        }
    }
}
