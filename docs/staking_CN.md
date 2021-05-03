# Staking

## Validator 变更

- 基于多重签名机制实现 Validator 的信息变理
    - 添加一种新的交易(Operation)类型, 记为 ValidatorUpdate
    - 合法的 ValidatorUpdate 交易, 其签名权重必须达到经济模型中定义的阀值
        - 如: `>= (70% * [validator vote power].sum())`
    - 合法的 ValidatorUpdate 交易会触发 Validator 集合的变更
        - 候选名单中权重排名前 N 的节点, 成为正式的 Validator
            - N 的值由经济模型定义
        - 在 Tendermint 框架下, 从 H + 2 高度开始生效, H 指当前块高度
- 初始的静态 Validator 信息使用硬编码的方式指定
    - 都是 Findora 基金会的节点

## FRA 质押

- 添加一种新的交易(Operation)类型, 记为 Delegation
- 合法的 Delegation 交易中, 只能存在三个 Operation
    1. Delegation Operation 本身
    2. 支付交易费的 Transfer Operation
    3. 向自己转账的 Transfer Operation
        - 转账类型必须为明文
        - OutPut 中的 UTXO 必须只有一个
        - 该 UTXO 中的数额将作为质押的金额使用
- Delegation 交易一旦在链上达成共识, 对应的账户地址会被锁定
    - 质押期(包括质押结束之后的冻结期)内禁止一切资产转出行为
    - 允许从外部向质押地址中转入资产
- 另添加一种新的交易(Operation)类型, 记为 UnDelegation
    - 用于支持解除质押

## 链上治理

- 所有 Validator(包括候选者) 都必须提供一个收益接收地址
    - 记为 VRA(validator rewards address)
    - VRA 必须处于质押状态, 否则其投票权重将被置为 0
        - 预定义的初始 validator 集合不受此限制
    - Validator 获取的所有链上收益, 自动转入 VRA
        - 如: 出块奖励、质押收益、交易收续费等
- 恶意行为的链上举证和判定, 基于多重签名机制实现
    - 添加一种新的交易(Operation)类型, 记为 Governance
    - Governance 的交易体中载有恶意行为的详细信息
        - 如: 恶意节点身份、行为类型、块高度及相关的交易等
    - 合法的 Governance 交易, 其签名权重必须达到经济模型中定义的阀值
        - 如: `>= (70% * [validator vote power].sum())`
    - 合法的 Governance 交易会触发针对恶意节点的资产罚没规则
        - 根据经济模型中的定义, 罚没其 VRA 中的部分或全部资产
- 发现恶意行为的途径
    - Tendermint 可自动识别的部分, 在链上自动处罚
    - 线下人工发现的恶意行为, 通过发送 Governance 交易进行处罚
        - 需要足够多的 Validator 签名

## FRA 官方发币

- 添加一种新的交易(Operation)类型, 记为 FraDistribution
    - 合法的 FraDistribution 必须达到预定义的多重签名的权重阀值
    - 此 Operation 实质上是向系统中注册新的发币信息
    - 实际的发币交易, 将由 CoinBase 自动发出

## 出块奖励

- 上线前期可使用静态值
    - 如: 每个块固定为 100 FRA
- 后续可升级为更完备的计算逻辑

## 收益分发

- 系统在链上维护一份全局收益列表
    - 该列表中的数据参与集群共识
- 每隔 N 个块发放一次收益
    - N 的值由经济模型定义, 默认为 1
- 定义一个完全公开的收益分发地址
    - 记为 CoinBase
    - CoinBase 的私钥是公开存储于链上的
- 使用 CoinBase 实现收益的自动分发
    - 只允许从 CoinBase 向系统收益列表中的地址转账
        - 转账类型必须是明文
        - 数额必须完全匹配
    - CoinBase 自身不发行任何资产
    - 其余额来源于线下的人工充值和交易手续费
        - 目前的静态手续费的接收地址将由'黑洞'改为 CoinBase
    - 余额不足时, 将在下一次获得充值时补发
    - Tips: 此为针对当前 FRA 发行方式的填坑策略
- 官方发币、出块奖励、质押收益等均通过 CoinBase 自动分发
