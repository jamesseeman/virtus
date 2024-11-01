use anyhow::Result;
use nftables::batch::Batch;
use nftables::expr::Expression;
use nftables::schema::{Chain, NfListObject, NfObject::ListObject, Rule, Table};
use nftables::stmt::{Match, Operator, Statement, NAT};
use nftables::types::{NfChainPolicy, NfChainType, NfFamily, NfHook};

fn main() -> Result<()> {
    println!("{:?}", get_tables()?);
    //println!("{:?}", get_chains()?);
    //println!("{:?}", get_rules()?);
    //println!("{:?}", get_maps()?);

    create_table("virtus")?;
    println!("{:?}", get_tables()?);

    //type nat hook postrouting priority srcnat; policy accept;
    create_chain(
        "POSTROUTING",
        "virtus",
        Some(NfChainType::NAT),
        Some(NfHook::Postrouting),
        Some(100),
        Some(NfChainPolicy::Accept),
    )?;
    println!("{:?}", get_chains()?);

    // Rule(Rule { family: IP, table: "nat", chain: "POSTROUTING", expr: [Match(Match { left: Named(Meta(Meta { key: Oifname })), right: String("docker0"), op: NEQ }), Match(Match { left: Named(Payload(PayloadField(PayloadField { protocol: "ip", field: "saddr" }))), right: Named(Prefix(Prefix { addr: String("172.17.0.0"), len: 16 })), op: EQ }), Counter(Some(Counter { packets: Some(85), bytes: Some(5262) })), XT(None)], handle: Some(36), index: None, comment: None })

    // oifname != "docker0" ip saddr 172.17.0.0/16 counter packets 85 bytes 5262 masquerade
    create_rule(
        "virtus",
        "POSTROUTING",
        vec![
            Statement::Match(Match {
                left: Expression::Named(nftables::expr::NamedExpression::Meta(
                    nftables::expr::Meta {
                        key: nftables::expr::MetaKey::Oifname,
                    },
                )),
                right: Expression::String(String::from("virtus-br")),
                op: Operator::NEQ,
            }),
            Statement::Match(Match {
                left: Expression::Named(nftables::expr::NamedExpression::Payload(
                    nftables::expr::Payload::PayloadField(nftables::expr::PayloadField {
                        protocol: String::from("ip"),
                        field: String::from("saddr"),
                    }),
                )),
                right: Expression::Named(nftables::expr::NamedExpression::Prefix(
                    nftables::expr::Prefix {
                        addr: Box::new(Expression::String(String::from("172.100.0.0"))),
                        len: 16,
                    },
                )),
                op: Operator::EQ,
            }),
            Statement::Masquerade(Some(NAT {
                family: None,
                addr: None,
                port: None,
                flags: None,
            })),
        ],
    )?;

    Ok(())
}

fn create_table(name: &str) -> Result<()> {
    let mut batch = Batch::new();
    batch.add(NfListObject::Table(Table::new(
        NfFamily::IP,
        String::from(name),
    )));
    nftables::helper::apply_ruleset(&batch.to_nftables(), None, None)?;
    Ok(())
}

fn create_chain(
    name: &str,
    table: &str,
    _type: Option<NfChainType>,
    hook: Option<NfHook>,
    priority: Option<i32>,
    policy: Option<NfChainPolicy>,
) -> Result<()> {
    let mut batch = Batch::new();
    batch.add(NfListObject::Chain(Chain::new(
        NfFamily::IP,
        String::from(table),
        String::from(name),
        _type,
        hook,
        priority,
        None,
        policy,
    )));
    nftables::helper::apply_ruleset(&batch.to_nftables(), None, None)?;
    Ok(())
}

fn create_rule(table: &str, chain: &str, expr: Vec<Statement>) -> Result<()> {
    let mut batch = Batch::new();
    batch.add(NfListObject::Rule(Rule::new(
        NfFamily::IP,
        String::from(table),
        String::from(chain),
        expr,
    )));
    nftables::helper::apply_ruleset(&batch.to_nftables(), None, None)?;
    Ok(())
}

fn get_tables() -> Result<Vec<NfListObject>> {
    let ruleset = nftables::helper::get_current_ruleset(None, None)?;
    let tables = ruleset
        .objects
        .into_iter()
        .filter_map(|object| match object {
            ListObject(NfListObject::Table(table)) => Some(NfListObject::Table(table)),
            _ => None,
        })
        .collect();

    Ok(tables)
}

fn get_chains() -> Result<Vec<NfListObject>> {
    let ruleset = nftables::helper::get_current_ruleset(None, None)?;
    let chains = ruleset
        .objects
        .into_iter()
        .filter_map(|object| match object {
            ListObject(NfListObject::Chain(chain)) => Some(NfListObject::Chain(chain)),
            _ => None,
        })
        .collect();

    Ok(chains)
}

fn get_rules() -> Result<Vec<NfListObject>> {
    let ruleset = nftables::helper::get_current_ruleset(None, None)?;
    let rules = ruleset
        .objects
        .into_iter()
        .filter_map(|object| match object {
            ListObject(NfListObject::Rule(rule)) => Some(NfListObject::Rule(rule)),
            _ => None,
        })
        .collect();

    Ok(rules)
}

fn get_maps() -> Result<Vec<NfListObject>> {
    let ruleset = nftables::helper::get_current_ruleset(None, None)?;
    let maps = ruleset
        .objects
        .into_iter()
        .filter_map(|object| match object {
            ListObject(NfListObject::Map(map)) => Some(NfListObject::Map(map)),
            _ => None,
        })
        .collect();

    Ok(maps)
}
