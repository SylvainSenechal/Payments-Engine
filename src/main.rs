use serde::Deserialize;

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

// https://docs.rs/csv/1.1.6/csv/tutorial/index.html
fn main() -> std::io::Result<()> {
    let transactions = read_from_file("src/testSamples/input1.csv")?;
    print!("{:?}", transactions);

    Ok(())
}

fn read_from_file(file_path: &str) -> Result<Vec<Transaction>, csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file_path)?;

    rdr.deserialize()
        .collect::<Result<Vec<Transaction>, csv::Error>>()
}
