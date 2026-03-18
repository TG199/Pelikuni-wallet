

#derive[Clone, Debug]
pub struct WalletService {
    database: PgPool,
    repository: WalletRepository,
}

impl WalletService {
    pub async fn create_wallet(self, request: CreateWalletRequest) -> Result<WalletResponse, anyhow::Error> {
        let wallet = self.repository.create_wallet(

        )
    }
}

pub struct Wallet {
    user_id: Uuid,
    balance: f64,
    version: f64,
}

pub struct WalletRepository;

impl WalletRepository {
    pub async fn create_wallet() -> Wallet {
        let wallet = Wallet {
            user_id: Uuid::new_v4(),
            balance: 0.0,
            version: 1.0,
        }
        wallet
    }
}