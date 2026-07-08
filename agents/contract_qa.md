# Contract QA

## 目的

schema、Forge Mapping、ATO Mapping、handoff、trace key が揃っているか検査する。

## 出力

- schema validation verdict
- mapping gap
- handoff key gap
- proof coverage gap
- repair task recommendation

## 境界

contract gap は原則 AI repair。Scope や authority の拡張が必要な場合だけ Human Decision Packet にする。
