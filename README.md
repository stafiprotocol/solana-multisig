# solana-multisig

`review后修改建议(given by 文军)`
1. create_multisig: 
    1. owners 可以增加长度判断
    2. threshold可以要求大于0
    3. nonce可以考虑设置为默认值比如说0呢，就不需要作为参数传入。
2. struct Transaction: 
    1. Transaction可以改为BatchTransaction, 理由是更直观
    2. program_id, data可以改为program_ids、datas, 当然如果修改的话相应的function里的参数可能也需要改一下
    3. did_execute可以改为executed， 理由是bool值用形容词更好， 并且形容词可以不加前缀is, eg：struct TransactionAccount的is_writable可以改为writable。不过加is也没问题。
    4. signers当前是Vec, 可以考虑改为用map。 另外就是可以通过len或者是增加一个sig_count字段来记录签名数量从而避免在approve里通过循环来获取该值。
3. create_transaction: 
    1. 参数pid、data建议改为pids、datas, 因为是数组。
    2. 一笔交易包含三个元素：pid、acc、data, 可不可以构建一个struct包含这三个元素，eg: SingleTransaction, 然后参数就只有一个transactions: Vec<SingleTransaction>, 这样就不需要再担心size/len的问题了。每个交易本身也是独立的
    3. 另外就是create_transaction本身也是一个approve， 如果threshold值是1， 那按照预期在create的同时是就应该去execute;
4. 功能讨论： 当前multisig create之后似乎不支持修改， 比如add_owner, remove_owner 以及threshold的修改
