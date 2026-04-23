#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, // Đã thêm contracterror
    log, panic_with_error, Address, Env, String, Vec,
};

// ─────────────────────────────────────────────
// Error codes
// ─────────────────────────────────────────────
#[contracterror] // Đổi từ #[contracttype] sang #[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MarketError {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    Unauthorized       = 3,
    ItemNotFound       = 4,
    ItemNotActive      = 5,
    ItemAlreadySold    = 6,
    InvalidPrice       = 7,
    InvalidInput       = 8,
    CannotBuyOwn       = 9,
}
// Đã xoá block "impl IntoVal..." thủ công vì #[contracterror] đã tự động làm việc này.

// ─────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartCategory {
    Drivetrain,
    Brakes,
    Wheels,
    Handlebars,
    Saddle,
    Frame,
    Lighting,
    Accessories,
    Other,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartCondition {
    New,
    LikeNew,
    Good,
    Fair,
    ForParts,
}

// ─────────────────────────────────────────────
// Structs
// ─────────────────────────────────────────────
#[contracttype]
#[derive(Clone, Debug)]
pub struct PartListing {
    pub id: u64,
    pub seller: Address,
    pub name: String,
    pub description: String,
    pub category: PartCategory,
    pub condition: PartCondition,
    pub price_stroops: i128,
    pub is_active: bool,
    pub buyer: Option<Address>,
}

// ─────────────────────────────────────────────
// Storage keys
// ─────────────────────────────────────────────
#[contracttype]
pub enum DataKey {
    Admin,
    ListingCount,
    Listing(u64),
}

// ─────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────
#[contract]
pub struct BicycleMarket;

#[contractimpl]
impl BicycleMarket {

    /// Khởi tạo marketplace - gọi 1 lần duy nhất
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, MarketError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ListingCount, &0u64);
        log!(&env, "Marketplace initialized, admin: {}", admin);
    }

    /// Đăng bán 1 phụ tùng xe đạp
    pub fn list_item(
        env: Env,
        seller: Address,
        name: String,
        description: String,
        category: PartCategory,
        condition: PartCondition,
        price_stroops: i128,
    ) -> u64 {
        seller.require_auth();

        if price_stroops <= 0 {
            panic_with_error!(&env, MarketError::InvalidPrice);
        }
        if name.len() == 0 {
            panic_with_error!(&env, MarketError::InvalidInput);
        }

        let count: u64 = env.storage().instance()
            .get(&DataKey::ListingCount).unwrap_or(0);
        let new_id = count + 1;

        let listing = PartListing {
            id: new_id,
            seller: seller.clone(),
            name: name.clone(),
            description,
            category,
            condition,
            price_stroops,
            is_active: true,
            buyer: None,
        };

        env.storage().persistent().set(&DataKey::Listing(new_id), &listing);
        env.storage().instance().set(&DataKey::ListingCount, &new_id);

        log!(&env, "Listed item #{}: {}", new_id, name);
        new_id
    }

    /// Mua sản phẩm (đánh dấu đã bán)
    pub fn buy_item(env: Env, buyer: Address, listing_id: u64) {
        buyer.require_auth();

        let mut listing: PartListing = env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic_with_error!(&env, MarketError::ItemNotFound));

        if !listing.is_active {
            panic_with_error!(&env, MarketError::ItemNotActive);
        }
        if listing.buyer.is_some() {
            panic_with_error!(&env, MarketError::ItemAlreadySold);
        }
        if listing.seller == buyer {
            panic_with_error!(&env, MarketError::CannotBuyOwn);
        }

        listing.is_active = false;
        listing.buyer = Some(buyer.clone());
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);

        log!(&env, "Item #{} bought by {}", listing_id, buyer);
    }

    /// Gỡ sản phẩm khỏi danh sách
    pub fn unlist_item(env: Env, seller: Address, listing_id: u64) {
        seller.require_auth();

        let mut listing: PartListing = env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic_with_error!(&env, MarketError::ItemNotFound));

        if listing.seller != seller {
            panic_with_error!(&env, MarketError::Unauthorized);
        }
        if !listing.is_active {
            panic_with_error!(&env, MarketError::ItemNotActive);
        }

        listing.is_active = false;
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
        log!(&env, "Item #{} unlisted", listing_id);
    }

    /// Cập nhật giá sản phẩm
    pub fn update_price(env: Env, seller: Address, listing_id: u64, new_price: i128) {
        seller.require_auth();
        if new_price <= 0 {
            panic_with_error!(&env, MarketError::InvalidPrice);
        }

        let mut listing: PartListing = env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic_with_error!(&env, MarketError::ItemNotFound));

        if listing.seller != seller {
            panic_with_error!(&env, MarketError::Unauthorized);
        }
        if !listing.is_active {
            panic_with_error!(&env, MarketError::ItemNotActive);
        }

        listing.price_stroops = new_price;
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
        log!(&env, "Item #{} price updated to {}", listing_id, new_price);
    }

    // ─── READ ONLY ────────────────────────────

    pub fn get_item(env: Env, listing_id: u64) -> PartListing {
        env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic_with_error!(&env, MarketError::ItemNotFound))
    }

    pub fn get_listing_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::ListingCount).unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, MarketError::NotInitialized))
    }

    pub fn get_active_listings(env: Env) -> Vec<PartListing> {
        let count: u64 = env.storage().instance()
            .get(&DataKey::ListingCount).unwrap_or(0);
        let mut result: Vec<PartListing> = Vec::new(&env); // Đã đổi cú pháp tạo Vec
        for i in 1..=count {
            if let Some(listing) = env.storage().persistent().get::<DataKey, PartListing>(&DataKey::Listing(i))
            {
                if listing.is_active {
                    result.push_back(listing);
                }
            }
        }
        result
    }

    pub fn get_listings_by_category(env: Env, category: PartCategory) -> Vec<PartListing> {
        let count: u64 = env.storage().instance()
            .get(&DataKey::ListingCount).unwrap_or(0);
        let mut result: Vec<PartListing> = Vec::new(&env); // Đã đổi cú pháp tạo Vec
        for i in 1..=count {
            if let Some(listing) = env.storage().persistent().get::<DataKey, PartListing>(&DataKey::Listing(i))
            {
                if listing.is_active && listing.category == category {
                    result.push_back(listing);
                }
            }
        }
        result
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Address, String};

    fn setup() -> (Env, BicycleMarketClient<'static>, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin  = Address::generate(&env);
        let seller = Address::generate(&env);
        let buyer  = Address::generate(&env);
        
        // Đã sửa cú pháp register_contract chuẩn cho Soroban mới
        let id = env.register_contract(None, BicycleMarket);
        let client = BicycleMarketClient::new(&env, &id);
        
        client.initialize(&admin);
        (env, client, admin, seller, buyer)
    }

    #[test]
    fn test_list_and_get() {
        let (env, client, _, seller, _) = setup();
        let item_id = client.list_item(
            &seller,
            &String::from_str(&env, "Shimano 105 Chain"),
            &String::from_str(&env, "11 speed, new"),
            &PartCategory::Drivetrain,
            &PartCondition::New,
            &50_000_000i128,
        );
        assert_eq!(item_id, 1);
        let item = client.get_item(&1u64);
        assert!(item.is_active);
        assert_eq!(item.buyer, None);
    }

    #[test]
    fn test_buy_item() {
        let (env, client, _, seller, buyer) = setup();
        let item_id = client.list_item(
            &seller,
            &String::from_str(&env, "Continental Tire"),
            &String::from_str(&env, "700x28c"),
            &PartCategory::Wheels,
            &PartCondition::New,
            &80_000_000i128,
        );
        client.buy_item(&buyer, &item_id);
        let item = client.get_item(&item_id);
        assert!(!item.is_active);
        assert_eq!(item.buyer, Some(buyer));
    }

    #[test]
    fn test_unlist() {
        let (env, client, _, seller, _) = setup();
        let item_id = client.list_item(
            &seller,
            &String::from_str(&env, "FSA Handlebar"),
            &String::from_str(&env, "420mm"),
            &PartCategory::Handlebars,
            &PartCondition::Good,
            &120_000_000i128,
        );
        client.unlist_item(&seller, &item_id);
        let item = client.get_item(&item_id);
        assert!(!item.is_active);
    }

    #[test]
    fn test_update_price() {
        let (env, client, _, seller, _) = setup();
        let item_id = client.list_item(
            &seller,
            &String::from_str(&env, "Selle Saddle"),
            &String::from_str(&env, "Racing saddle"),
            &PartCategory::Saddle,
            &PartCondition::Good,
            &200_000_000i128,
        );
        client.update_price(&seller, &item_id, &150_000_000i128);
        let item = client.get_item(&item_id);
        assert_eq!(item.price_stroops, 150_000_000i128);
    }

    #[test]
    fn test_category_filter() {
        let (env, client, _, seller, _) = setup();
        client.list_item(&seller,
            &String::from_str(&env, "SRAM Chain"), &String::from_str(&env, "11sp"),
            &PartCategory::Drivetrain, &PartCondition::New, &20_000_000i128);
        client.list_item(&seller,
            &String::from_str(&env, "Brake Caliper"), &String::from_str(&env, "Road"),
            &PartCategory::Brakes, &PartCondition::New, &30_000_000i128);
        client.list_item(&seller,
            &String::from_str(&env, "SRAM Cassette"), &String::from_str(&env, "11-32T"),
            &PartCategory::Drivetrain, &PartCondition::LikeNew, &40_000_000i128);

        let dt = client.get_listings_by_category(&PartCategory::Drivetrain);
        assert_eq!(dt.len(), 2);
        let br = client.get_listings_by_category(&PartCategory::Brakes);
        assert_eq!(br.len(), 1);
    }
}