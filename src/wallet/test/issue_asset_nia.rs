use super::*;
use serial_test::parallel;

#[test]
#[parallel]
fn success() {
    initialize();

    let (mut wallet, online) = get_funded_wallet!();

    // add a pending operation to an UTXO so spendable balance will be != settled / future
    let _receive_data = wallet.blind_receive(
        None,
        None,
        None,
        TRANSPORT_ENDPOINTS.clone(),
        MIN_CONFIRMATIONS,
    );

    let before_timestamp = now().unix_timestamp();
    let asset = wallet
        .issue_asset_nia(
            online,
            TICKER.to_string(),
            NAME.to_string(),
            PRECISION,
            vec![AMOUNT, AMOUNT],
        )
        .unwrap();
    show_unspent_colorings(&wallet, "after issuance");
    assert_eq!(asset.ticker, TICKER.to_string());
    assert_eq!(asset.name, NAME.to_string());
    assert_eq!(asset.precision, PRECISION);
    assert_eq!(
        asset.balance,
        Balance {
            settled: AMOUNT * 2,
            future: AMOUNT * 2,
            spendable: AMOUNT * 2,
        }
    );
    assert!(before_timestamp <= asset.added_at && asset.added_at <= now().unix_timestamp());
}

#[test]
#[parallel]
fn multi_success() {
    initialize();

    let amounts: Vec<u64> = vec![111, 222, 333, 444, 555];
    let sum: u64 = amounts.iter().sum();

    let (mut wallet, online) = get_funded_wallet!();

    let asset = wallet
        .issue_asset_nia(
            online,
            TICKER.to_string(),
            NAME.to_string(),
            PRECISION,
            amounts.clone(),
        )
        .unwrap();

    // check balance is the sum of the amounts
    assert_eq!(asset.balance.settled, sum);

    // check each allocation ends up on a different UTXO
    let unspents: Vec<Unspent> = wallet
        .list_unspents(None, true)
        .unwrap()
        .into_iter()
        .filter(|u| !u.rgb_allocations.is_empty())
        .collect();
    let mut outpoints: Vec<Outpoint> = vec![];
    for unspent in &unspents {
        let outpoint = unspent.utxo.outpoint.clone();
        assert!(!outpoints.contains(&outpoint));
        outpoints.push(outpoint);
    }
    assert_eq!(outpoints.len(), amounts.len());

    // check all allocations are of the same asset
    assert!(unspents
        .iter()
        .filter(|u| !u.rgb_allocations.is_empty())
        .all(|u| {
            u.rgb_allocations.len() == 1
                && u.rgb_allocations.first().unwrap().asset_id == Some(asset.asset_id.clone())
        }));
}

#[test]
#[parallel]
fn no_issue_on_pending_send() {
    initialize();

    let amount: u64 = 66;

    let (mut wallet, online) = get_funded_wallet!();
    let (mut rcv_wallet, rcv_online) = get_funded_wallet!();

    // issue 1st asset
    let asset_1 = wallet
        .issue_asset_nia(
            online.clone(),
            TICKER.to_string(),
            NAME.to_string(),
            PRECISION,
            vec![AMOUNT],
        )
        .unwrap();
    // get 1st issuance UTXO
    let unspents = wallet.list_unspents(None, false).unwrap();
    let unspent_1 = unspents
        .iter()
        .find(|u| {
            u.rgb_allocations
                .iter()
                .any(|a| a.asset_id == Some(asset_1.asset_id.clone()))
        })
        .unwrap();
    // send 1st asset
    let receive_data = rcv_wallet
        .blind_receive(
            None,
            None,
            None,
            TRANSPORT_ENDPOINTS.clone(),
            MIN_CONFIRMATIONS,
        )
        .unwrap();
    let recipient_map = HashMap::from([(
        asset_1.asset_id.clone(),
        vec![Recipient {
            amount,
            recipient_data: RecipientData::BlindedUTXO(
                SecretSeal::from_str(&receive_data.recipient_id).unwrap(),
            ),
            transport_endpoints: TRANSPORT_ENDPOINTS.clone(),
        }],
    )]);
    let txid = test_send_default(&mut wallet, &online, recipient_map);
    assert!(!txid.is_empty());

    // issue 2nd asset
    let asset_2 = wallet
        .issue_asset_nia(
            online.clone(),
            s!("TICKER2"),
            s!("NAME2"),
            PRECISION,
            vec![AMOUNT * 2],
        )
        .unwrap();
    show_unspent_colorings(&wallet, "after 2nd issuance");
    // get 2nd issuance UTXO
    let unspents = wallet.list_unspents(None, false).unwrap();
    let unspent_2 = unspents
        .iter()
        .find(|u| {
            u.rgb_allocations
                .iter()
                .any(|a| a.asset_id == Some(asset_2.asset_id.clone()))
        })
        .unwrap();
    // check 2nd issuance was not allocated to the same UTXO as the 1st one (now being spent)
    assert_ne!(unspent_1.utxo.outpoint, unspent_2.utxo.outpoint);

    // progress transfer to WaitingConfirmations
    rcv_wallet.refresh(rcv_online, None, vec![]).unwrap();
    wallet
        .refresh(online.clone(), Some(asset_1.asset_id.clone()), vec![])
        .unwrap();
    // issue 3rd asset
    let asset_3 = wallet
        .issue_asset_nia(
            online,
            s!("TICKER3"),
            s!("NAME3"),
            PRECISION,
            vec![AMOUNT * 3],
        )
        .unwrap();
    show_unspent_colorings(&wallet, "after 3rd issuance");
    // get 3rd issuance UTXO
    let unspents = wallet.list_unspents(None, false).unwrap();
    let unspent_3 = unspents
        .iter()
        .find(|u| {
            u.rgb_allocations
                .iter()
                .any(|a| a.asset_id == Some(asset_3.asset_id.clone()))
        })
        .unwrap();
    // check 3rd issuance was not allocated to the same UTXO as the 1st one (now being spent)
    assert_ne!(unspent_1.utxo.outpoint, unspent_3.utxo.outpoint);
}

#[test]
#[parallel]
fn fail() {
    initialize();

    let (mut wallet, online) = get_funded_wallet!();

    // supply overflow
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        NAME.to_string(),
        PRECISION,
        vec![u64::MAX, u64::MAX],
    );
    assert!(matches!(result, Err(Error::TooHighIssuanceAmounts)));

    // bad online object
    let other_online = Online {
        id: 1,
        electrum_url: wallet.online_data.as_ref().unwrap().electrum_url.clone(),
    };
    let result = wallet.issue_asset_nia(
        other_online,
        TICKER.to_string(),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::CannotChangeOnline)));

    // invalid ticker: empty
    let result = wallet.issue_asset_nia(
        online.clone(),
        s!(""),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidTicker { details: m }) if m == IDENT_EMPTY_MSG));

    // invalid ticker: too long
    let result = wallet.issue_asset_nia(
        online.clone(),
        s!("ABCDEFGHI"),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidTicker { details: m }) if m == IDENT_TOO_LONG_MSG));

    // invalid ticker: lowercase
    let result = wallet.issue_asset_nia(
        online.clone(),
        s!("TiCkEr"),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(
        matches!(result, Err(Error::InvalidTicker { details: m }) if m == "ticker needs to be all uppercase")
    );

    // invalid ticker: unicode characters
    let result = wallet.issue_asset_nia(
        online.clone(),
        s!("TICKER WITH ℧NICODE CHARACTERS"),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidTicker { details: m }) if m == IDENT_NOT_ASCII_MSG));

    // invalid name: empty
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        s!(""),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidName { details: m }) if m == IDENT_EMPTY_MSG));

    // invalid name: too long
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        ("a").repeat(257),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidName { details: m }) if m == IDENT_TOO_LONG_MSG));

    // invalid name: unicode characters
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        s!("name with ℧nicode characters"),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InvalidName { details: m }) if m == IDENT_NOT_ASCII_MSG));

    // invalid precision
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        NAME.to_string(),
        19,
        vec![AMOUNT],
    );
    assert!(matches!(
        result,
        Err(Error::InvalidPrecision { details: m }) if m == "precision is too high"
    ));

    // invalid amount list
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        NAME.to_string(),
        19,
        vec![],
    );
    assert!(matches!(result, Err(Error::NoIssuanceAmounts)));

    drain_wallet(&wallet, online.clone());

    // insufficient funds
    let result = wallet.issue_asset_nia(
        online.clone(),
        TICKER.to_string(),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(
        result,
        Err(Error::InsufficientBitcoins {
            needed: _,
            available: _
        })
    ));

    fund_wallet(wallet.get_address());
    mine(false);

    // insufficient allocations
    let result = wallet.issue_asset_nia(
        online,
        TICKER.to_string(),
        NAME.to_string(),
        PRECISION,
        vec![AMOUNT],
    );
    assert!(matches!(result, Err(Error::InsufficientAllocationSlots)));
}
