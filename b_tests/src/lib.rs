mod b_generated;
mod generated;

#[cfg(test)]
mod test {
    use super::*;
    use prost::Message;
    use std::str::FromStr;
    use uuid::Uuid;

    const UUID: &'static str = "cd663747-6cb1-4ddc-bdfe-3dc76db62724";

    fn get_uuid() -> Uuid {
        uuid::Uuid::from_str(UUID).unwrap()
    }

    fn get_no_uuid() -> String {
        "no_uuid".to_string()
    }

    fn get_custom() -> String {
        "custom".to_string()
    }

    fn get_amount() -> i32 {
        1
    }

    fn b_generated_order() -> b_generated::Order {
        b_generated::Order {
            gender: b_generated::Gender::Female,
            genders: vec![b_generated::Gender::Female, b_generated::Gender::Other],
            currency: Some(b_generated::Currency {
                c: Some(b_generated::currency::C::Amount(get_amount())),
            }),
            o_currency: None,
            currencies: vec![
                b_generated::Currency {
                    c: Some(b_generated::currency::C::Custom(get_custom())),
                },
                b_generated::Currency {
                    c: Some(b_generated::currency::C::Amount(get_amount())),
                },
            ],
            uuid: get_uuid(),
            no_uuid: get_no_uuid(),
            repeated_uuids: vec![get_uuid(), get_uuid()],
            no_uuids: vec![get_custom()],
            order_inner: b_generated::order::OrderInner::InnerAnother,
            order_inners: vec![
                b_generated::order::OrderInner::InnerAnother,
                b_generated::order::OrderInner::InnerAnother2,
            ],
            something: Some(b_generated::order::Something::AlsoUuid(get_uuid())),
        }
    }

    fn generated_order() -> generated::Order {
        generated::Order {
            gender: generated::Gender::Female as i32,
            genders: vec![
                generated::Gender::Female as i32,
                generated::Gender::Other as i32,
            ],
            currency: Some(generated::Currency {
                c: Some(generated::currency::C::Amount(get_amount())),
            }),
            o_currency: None,
            currencies: vec![
                generated::Currency {
                    c: Some(generated::currency::C::Custom(get_custom())),
                },
                generated::Currency {
                    c: Some(generated::currency::C::Amount(get_amount())),
                },
            ],
            uuid: get_uuid().to_string(),
            no_uuid: get_no_uuid(),
            repeated_uuids: vec![get_uuid().to_string(), get_uuid().to_string()],
            no_uuids: vec![get_custom()],
            order_inner: generated::order::OrderInner::InnerAnother as i32,
            order_inners: vec![
                generated::order::OrderInner::InnerAnother as i32,
                generated::order::OrderInner::InnerAnother2 as i32,
            ],
            something: Some(generated::order::Something::AlsoUuid(
                get_uuid().to_string(),
            )),
        }
    }

    #[test]
    fn equal() {
        let g = b_generated_order();
        let b = generated_order();
        let g = g.encode_buffer().unwrap();
        let b = b.encode_buffer().unwrap();

        assert_eq!(g, b);
        assert_eq!(g.encoded_len(), b.encoded_len());

        // Check if encoding works
        b_generated::Order::decode(b.as_slice()).unwrap();
        generated::Order::decode(g.as_slice()).unwrap();
    }

    fn check_order(order: generated::Order) {
        let b = order.encode_buffer().unwrap();

        b_generated::Order::decode(b.as_slice()).unwrap();
    }

    macro_rules! write_invalid_test {
        ($method_name: ident, $order: ident, $change: tt) => {
            #[test]
            #[should_panic]
            fn $method_name() {
                let mut $order = generated_order();

                $change;

                check_order($order);
            }
        };
    }

    write_invalid_test!(
        invalid_gender_zero,
        order,
        ({
            order.gender = 0;
        })
    );
    write_invalid_test!(
        invalid_gender_over_max,
        order,
        ({
            order.gender = 999;
        })
    );
    write_invalid_test!(
        invalid_uuids,
        order,
        ({
            order.repeated_uuids = vec![get_uuid().to_string(), get_no_uuid().to_string()];
        })
    );
    write_invalid_test!(
        empty_currency,
        order,
        ({
            order.currency = None;
        })
    );
    write_invalid_test!(
        invalid_uuid,
        order,
        ({
            order.uuid = order.uuid[1..].to_string();
        })
    );

    write_invalid_test!(
        invalid_inner_zero,
        order,
        ({
            order.order_inner = 0;
        })
    );
    write_invalid_test!(
        invalid_inner_over_max,
        order,
        ({
            order.order_inner = 999;
        })
    );
    write_invalid_test!(
        empty_something,
        order,
        ({
            order.something = None;
        })
    );
    write_invalid_test!(
        something_invalid_uuid,
        order,
        ({
            order.something = Some(generated::order::Something::AlsoUuid(get_no_uuid()));
        })
    );
}
