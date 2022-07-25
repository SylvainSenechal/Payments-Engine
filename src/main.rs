use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

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

// 4 digit precision voir {:4} ?
// map error
// handle empty column csv, or wrong type
// check tx or client id out of u16/u32
// handle double dispute / double resolve
// https://docs.rs/csv/1.1.6/csv/tutorial/index.html
// voir macro, errors, ?, sync/send, trait
fn main() -> std::io::Result<()> {
    let transactions = read_from_file("src/testSamples/input4.csv")?;
    let mut clients: HashMap<u16, Client> = HashMap::new();

    process_transactions(&transactions, &mut clients);
    write_clients_state(clients)?;

    Ok(())
}

fn read_from_file(file_path: &str) -> Result<Vec<Transaction>, csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file_path)?;

    rdr.deserialize()
        .collect::<Result<Vec<Transaction>, csv::Error>>()
}

fn write_clients_state(clients: HashMap<u16, Client>) -> Result<(), std::io::Error> {
    // see https://nnethercote.github.io/perf-book/io.html
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "client,available,held,total,locked")?;
    for (client_id, client) in clients {
        writeln!(
            lock,
            "{},{},{},{},{}",
            client_id, client.available, client.held, client.total, client.locked
        )?;
    }
    Ok(())
}

fn process_transactions(transactions: &Vec<Transaction>, clients: &mut HashMap<u16, Client>) {
    let mut transactions_history: HashMap<u32, Transaction> = HashMap::new();
    let mut ongoing_disputes: HashSet<u32> = HashSet::new();
    for t in transactions {
        // Get client of the transaction + initialize if it doesn't exists
        let client = clients.entry(t.client_id).or_default();

        if !client.locked {
            match t.category {
                TransactionCategory::Deposit => {
                    deposit(t.amount.unwrap(), client);
                    transactions_history.insert(t.tx, t.to_owned());
                }
                TransactionCategory::Withdrawal => {
                    if withdraw(t.amount.unwrap(), client) == true {
                        transactions_history.insert(t.tx, t.to_owned());
                    }
                }
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
        }
    }
}

fn deposit(amount: f32, client: &mut Client) {
    client.available += amount;
    client.total += amount;
}

fn withdraw(amount: f32, client: &mut Client) -> bool {
    if amount < client.available {
        client.available -= amount;
        client.total -= amount;
        return true;
    }
    return false;
}

fn dispute(
    transaction_disputed_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    // Can't dispute twice the same transaction
    if !ongoing_disputes.contains(&transaction_disputed_id) {
        // Can't dispute a transaction that doesn't exists
        if let Some(disputed) = transactions_history.get(&transaction_disputed_id) {
            match disputed.category {
                TransactionCategory::Deposit => {
                    client.available -= disputed.amount.unwrap();
                    client.held += disputed.amount.unwrap();
                    ongoing_disputes.insert(disputed.tx);
                }
                _ => (), // Not sure how to handle dispute on the other kind of transactions
            }
        }
    }
}

fn resolve(
    transaction_resolved_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    // Can't resolve a transaction that isn't under dispute
    if ongoing_disputes.contains(&transaction_resolved_id) {
        if let Some(resolved) = transactions_history.get(&transaction_resolved_id) {
            match resolved.category {
                TransactionCategory::Deposit => {
                    client.available += resolved.amount.unwrap();
                    client.held -= resolved.amount.unwrap();
                    ongoing_disputes.remove(&resolved.tx);
                }
                _ => (),
            }
        }
    }
}

fn charge_back(
    transaction_charged_back_id: u32,
    transactions_history: &HashMap<u32, Transaction>,
    ongoing_disputes: &mut HashSet<u32>,
    client: &mut Client,
) {
    if ongoing_disputes.contains(&transaction_charged_back_id) {
        if let Some(charged_back) = transactions_history.get(&transaction_charged_back_id) {
            match charged_back.category {
                TransactionCategory::Deposit => {
                    client.held -= charged_back.amount.unwrap();
                    client.total -= charged_back.amount.unwrap();
                    client.locked = true;
                    ongoing_disputes.remove(&charged_back.tx);
                }
                _ => (),
            }
        }
    }
}
