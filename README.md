# Applying Delf-rs to Web Submission System

[![Docs](https://img.shields.io/badge/docs-stable-blue.svg)](https://mcmcgrath13.github.io/delf-rs/delf/index.html)

A [DelF](https://cs.brown.edu/courses/csci2390/2020/readings/delf.pdf) inspired deletion framework in Rust.

delf-rs (the framework from which we forked from) is a lighter weight deletion framework that resembles Meta's DelF. The original DelF is a deletion framework created at Facebook, which aims to robustly delete all related data from its data stores as defined by the `deletion` definitions added to Facebook's existing data definition language (DDL).

## Running the code

### Prerequisites

* Rust
* MySQL

### Get Data

Step 1: Create a database named `myclass`. Instructions on how to do so can be viewed here: https://www.mysqltutorial.org/mysql-create-database/

Step 2: Import sample dataset by executing the following command. Note, `<username>` should be replaced with the user that has access to the `myclass` database. When we ran our code, our `<username>` was `root`.

```
mysql -u <username> -p myclass < data.txt
```

### Delf

Build the executable:

```
cargo build
```

Validate the websubmit schema

```
./target/debug/delf -s examples/websubmit-rs/schema.yaml -c examples/websubmit-rs/config.yaml validate
```

Run the API

```
./target/debug/delf -s examples/websubmit-rs/schema.yaml -c examples/websubmit-rs/config.yaml run

```

Sample deletion requests:

After deleting the following lecture object, all the related data rows in lectures, questions, and answers table will be deleted.

```
curl -X "DELETE" http://localhost:8000/object/lectures/1

```

To test another object deletion, you can restore the data by repeating the steps in the "Get Data" section.

After deleting the following users object, all the related data rows in users and answers table will be deleted.

```
curl -X "DELETE" http://localhost:8000/object/users/kate_nelson@brown.edu
```
