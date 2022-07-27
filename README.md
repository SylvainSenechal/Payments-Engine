# Payments-Engine

Run with ```cargo run -- src/testSamples/providedExample.csv > accounts.csv```

# Discussions

- We might need to use a crate that handles well decimal numbers to avoid rounding problems 

- Disputes, Resolves and Chargebacks only deals with Deposits, maybe we could've done something for the withdraws ?

- More tests are needed around floating precisions, and on large files > 1GB
