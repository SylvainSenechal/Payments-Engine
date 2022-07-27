use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::io::Write;

#[derive(Deserialize, Clone, Debug)]
struct Transaction {
    #[serde(rename = "type")]
    category: TransactionCategory,
    #[serde(rename = "client")]
    client_id: u16,
    tx: u32,
    amount: Option<f64>,
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
    available: f64,
    held: f64,
    total: f64,
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

fn main() -> Result<(), Box<dyn Error>> {
    let transactions = get_transactions_from_args()?;
    let mut clients: HashMap<u16, Client> = HashMap::new();
    process_transactions(&transactions, &mut clients)?;
    write_clients_state(&clients)?;

    Ok(())
}

fn get_transactions_from_args() -> Result<Vec<Transaction>, csv::Error> {
    let file_path = env::args()
        .nth(1)
        .expect("Please provide the csv file path as the first argument");
    get_transactions_from_file(&file_path)
}

fn get_transactions_from_file(file_path: &str) -> Result<Vec<Transaction>, csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file_path)?;

    rdr.deserialize()
        .collect::<Result<Vec<Transaction>, csv::Error>>()
}

fn write_clients_state(clients: &HashMap<u16, Client>) -> Result<(), std::io::Error> {
    // See https://nnethercote.github.io/perf-book/io.html
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "client,available,held,total,locked")?;
    for (client_id, client) in clients {
        writeln!(
            lock,
            "{},{:.4},{:.4},{:.4},{}",
            client_id, client.available, client.held, client.total, client.locked
        )?;
    }
    Ok(())
}

fn process_transactions(
    transactions: &Vec<Transaction>,
    clients: &mut HashMap<u16, Client>,
) -> Result<(), String> {
    let mut transactions_history: HashMap<u32, Transaction> = HashMap::new();
    let mut ongoing_disputes: HashSet<u32> = HashSet::new();
    for (csv_line, t) in transactions.iter().enumerate() {
        // Get client of the transaction, or initialize if it doesn't exists
        let client = clients.entry(t.client_id).or_default();

        if !client.locked {
            match t.category {
                TransactionCategory::Deposit => {
                    let amount = t.amount.expect(&format!("Incorrect csv row : {}. You should provide an amount for a deposit transaction", csv_line + 1));
                    deposit(amount, client)?;
                    transactions_history.insert(t.tx, t.to_owned());
                }
                TransactionCategory::Withdrawal => {
                    let amount = t.amount.expect(&format!("Incorrect csv row : {}. You should provide an amount for a withdraw transaction", csv_line + 1));
                    if withdraw(amount, client)? == true {
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

    Ok(())
}

fn deposit(amount: f64, client: &mut Client) -> Result<(), &str> {
    if amount < f64::MIN_POSITIVE {
        return Err("Cannot deposit a negative amount");
    }
    client.available += amount;
    client.total += amount;
    if client.total > f64::MAX {
        return Err("You are getting way too rich");
    }
    Ok(())
}

fn withdraw(amount: f64, client: &mut Client) -> Result<bool, &str> {
    if amount < f64::MIN_POSITIVE {
        return Err("Cannot withdraw a negative amount");
    }
    if amount < client.available {
        client.available -= amount;
        client.total -= amount;
        return Ok(true);
    }
    return Ok(false);
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
                    let amount = disputed.amount.expect(&format!(
                        "The amount of the disputed transaction number {} was not provided",
                        disputed.tx
                    ));
                    client.available -= amount;
                    client.held += amount;
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
                    let amount = resolved.amount.expect(&format!(
                        "The amount of the resolved transaction number {} was not provided",
                        resolved.tx
                    ));
                    client.available += amount;
                    client.held -= amount;
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
                    let amount = charged_back.amount.expect(&format!(
                        "The amount of the charged back transaction number {} was not provided",
                        charged_back.tx
                    ));
                    client.held -= amount;
                    client.total -= amount;
                    client.locked = true;
                    ongoing_disputes.remove(&charged_back.tx);
                }
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn invalid_input_amount_type() {
        get_transactions_from_file("src/testSamples/invalidAmountType.csv").unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_input_client_id() {
        get_transactions_from_file("src/testSamples/invalidClientID.csv").unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_input_transaction_id() {
        get_transactions_from_file("src/testSamples/invalidTransactionID.csv").unwrap();
    }

    #[test]
    #[should_panic]
    fn too_rich_client() {
        let transactions = get_transactions_from_file("src/testSamples/tooRichClient.csv").unwrap();
        process_transactions(&transactions, &mut HashMap::new()).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_input_unprovided_deposit_amount() {
        let transactions =
            get_transactions_from_file("src/testSamples/unprovidedAmount.csv").unwrap();
        process_transactions(&transactions, &mut HashMap::new()).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_input_negative_deposit() {
        let transactions =
            get_transactions_from_file("src/testSamples/negativeDeposit.csv").unwrap();
        process_transactions(&transactions, &mut HashMap::new()).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_input_negative_withdraw() {
        let transactions =
            get_transactions_from_file("src/testSamples/negativeWithdraw.csv").unwrap();
        process_transactions(&transactions, &mut HashMap::new()).unwrap();
    }

    #[test]
    fn provided_example() {
        let transactions =
            get_transactions_from_file("src/testSamples/providedExample.csv").unwrap();
        let mut clients: HashMap<u16, Client> = HashMap::new();
        process_transactions(&transactions, &mut clients).unwrap();

        assert_eq!(clients.get(&1).unwrap().available, 1.5);
        assert_eq!(clients.get(&1).unwrap().held, 0.0);
        assert_eq!(clients.get(&1).unwrap().total, 1.5);
        assert_eq!(clients.get(&1).unwrap().locked, false);

        assert_eq!(clients.get(&2).unwrap().available, 2.0);
        assert_eq!(clients.get(&2).unwrap().held, 0.0);
        assert_eq!(clients.get(&2).unwrap().total, 2.0);
        assert_eq!(clients.get(&2).unwrap().locked, false);
    }

    #[test]
    fn handle_dispute() {
        let transactions = get_transactions_from_file("src/testSamples/dispute.csv").unwrap();
        let mut clients: HashMap<u16, Client> = HashMap::new();
        process_transactions(&transactions, &mut clients).unwrap();

        assert_eq!(clients.get(&1).unwrap().available, 0.5);
        assert_eq!(clients.get(&1).unwrap().held, 1.0);
        assert_eq!(clients.get(&1).unwrap().total, 1.5);
        assert_eq!(clients.get(&1).unwrap().locked, false);

        assert_eq!(clients.get(&2).unwrap().available, 2.0);
        assert_eq!(clients.get(&2).unwrap().held, 0.0);
        assert_eq!(clients.get(&2).unwrap().total, 2.0);
        assert_eq!(clients.get(&2).unwrap().locked, false);
    }

    #[test]
    // Multiple dispute on the same transaction + dispute on a transaction that doesn't exists
    fn handle_tricky_disputes() {
        let transactions = get_transactions_from_file("src/testSamples/trickyDispute.csv").unwrap();
        let mut clients: HashMap<u16, Client> = HashMap::new();
        process_transactions(&transactions, &mut clients).unwrap();

        assert_eq!(clients.get(&1).unwrap().available, 0.5);
        assert_eq!(clients.get(&1).unwrap().held, 1.0);
        assert_eq!(clients.get(&1).unwrap().total, 1.5);
        assert_eq!(clients.get(&1).unwrap().locked, false);

        assert_eq!(clients.get(&2).unwrap().available, 2.0);
        assert_eq!(clients.get(&2).unwrap().held, 0.0);
        assert_eq!(clients.get(&2).unwrap().total, 2.0);
        assert_eq!(clients.get(&2).unwrap().locked, false);
    }

    #[test]
    // Multiple resolves on the same dispute + resolve on a dispute that doesn't exists
    fn handle_tricky_resolves() {
        let transactions = get_transactions_from_file("src/testSamples/trickyResolve.csv").unwrap();
        let mut clients: HashMap<u16, Client> = HashMap::new();
        process_transactions(&transactions, &mut clients).unwrap();

        assert_eq!(clients.get(&1).unwrap().available, 1.5);
        assert_eq!(clients.get(&1).unwrap().held, 0.0);
        assert_eq!(clients.get(&1).unwrap().total, 1.5);
        assert_eq!(clients.get(&1).unwrap().locked, false);

        assert_eq!(clients.get(&2).unwrap().available, 2.0);
        assert_eq!(clients.get(&2).unwrap().held, 0.0);
        assert_eq!(clients.get(&2).unwrap().total, 2.0);
        assert_eq!(clients.get(&2).unwrap().locked, false);
    }

    #[test]
    fn handle_charge_back() {
        let transactions = get_transactions_from_file("src/testSamples/chargeback.csv").unwrap();
        let mut clients: HashMap<u16, Client> = HashMap::new();
        process_transactions(&transactions, &mut clients).unwrap();

        assert_eq!(clients.get(&1).unwrap().available, 0.5);
        assert_eq!(clients.get(&1).unwrap().held, 0.0);
        assert_eq!(clients.get(&1).unwrap().total, 0.5);
        assert_eq!(clients.get(&1).unwrap().locked, true);

        assert_eq!(clients.get(&2).unwrap().available, 2.0);
        assert_eq!(clients.get(&2).unwrap().held, 0.0);
        assert_eq!(clients.get(&2).unwrap().total, 2.0);
        assert_eq!(clients.get(&2).unwrap().locked, false);
    }
}
