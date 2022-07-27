# Payments-Engine

If a Dispute, Resolve or Chargeback points towards a transaction that is not a Deposit, it is ignored.

We might need to use a crate that handles well decimal numbers to avoid rounding problems 

Handle better amount values that are larger than f64 bound

Using very, very large amount like 10 at the power of 400, will return "inf" amount

More tests to be performed on the float precisions, tests on the ouput format