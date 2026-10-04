#![allow(non_snake_case)]

use criterion::{Criterion, criterion_group};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wmi::{Variant, WMIConnection};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_Account")]
pub struct Account {
    pub __Path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_UserAccount")]
pub struct UserAccount {
    pub __Path: String,
    pub AccountType: i64,
    pub Caption: String,
    pub Description: String,
    pub Disabled: bool,
    pub Domain: String,
    pub FullName: String,
    pub LocalAccount: bool,
    pub Lockout: bool,
    pub Name: String,
    pub PasswordChangeable: bool,
    pub PasswordExpires: bool,
    pub PasswordRequired: bool,
    pub SID: String,
    pub SIDType: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_Group")]
pub struct Group {
    pub __Path: String,
    pub Name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_GroupUser")]
pub struct GroupUser {}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
pub struct Process {
    pub Caption: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "CIM_ProcessExecutable")]
pub struct ProcessExecutable {
    pub Antecedent: String,
    pub Dependent: String,
    pub BaseAddress: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Win32_Service")]
pub struct Service {
    pub Name: String,
    pub DisplayName: String,
    pub PathName: Option<String>,
    pub State: String,
    pub Started: bool,
}

fn get_accounts(con: &WMIConnection) {
    let _accounts: Vec<Account> = con.query().unwrap();
}

fn get_user_accounts(con: &WMIConnection) {
    let _users: Vec<UserAccount> = con.query().unwrap();
}

fn get_user_accounts_hash_map(con: &WMIConnection) {
    let _users: Vec<HashMap<String, Variant>> =
        con.raw_query("SELECT * FROM Win32_UserAccount").unwrap();
}

fn get_minimal_procs(con: &WMIConnection) {
    let _procs: Vec<Process> = con.query().unwrap();
}

fn get_procs_hash_map(con: &WMIConnection) {
    let _procs: Vec<HashMap<String, Variant>> =
        con.raw_query("SELECT * FROM Win32_Process").unwrap();
}

pub fn get_users_with_groups(con: &WMIConnection) {
    let mut filters = HashMap::new();

    filters.insert("Name".to_string(), "Administrators".into());

    let group: Group = con.filtered_query(&filters).unwrap().pop().unwrap();
    let _accounts: Vec<Account> = con
        .associators::<Account, GroupUser>(&group.__Path)
        .unwrap();
}

pub fn get_modules(con: &WMIConnection) {
    let _execs: Vec<ProcessExecutable> = con.query().unwrap();
}

fn get_services(con: &WMIConnection) {
    let _services: Vec<Service> = con.query().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    // baseline: 41ms
    c.bench_function("get_accounts", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_accounts(&wmi_con))
    });

    // baseline: 13ms
    c.bench_function("get_user_accounts", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_user_accounts(&wmi_con))
    });

    // baseline: 9ms
    c.bench_function("get_user_accounts_hash_map", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_user_accounts_hash_map(&wmi_con))
    });

    // baseline: 60ms
    c.bench_function("get_minimal_procs", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_minimal_procs(&wmi_con))
    });

    // baseline: 68ms
    c.bench_function("get_procs_hash_map", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_procs_hash_map(&wmi_con))
    });

    // baseline: 9s (**seconds**)
    // after adding AssocClass: 73ms
    c.bench_function("get_users_with_groups", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_users_with_groups(&wmi_con))
    });

    // baseline: 625ms.
    c.bench_function("get_modules", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_modules(&wmi_con))
    });

    // baseline: 300ms.
    c.bench_function("get_services", |b| {
        let wmi_con = WMIConnection::new().unwrap();
        b.iter(|| get_services(&wmi_con))
    });
}

criterion_group!(benches, criterion_benchmark);
fn main() {
    let mut c = Criterion::default().configure_from_args();

    // Uncomment when testing changes.
    // let mut c = c.sample_size(3);

    criterion_benchmark(&mut c);

    c.final_summary();
}
