# M-6000M用測定ソフトウェア

## 準備

### Linux(Debian)

シリアルポートへのアクセスはデフォルトでは一般ユーザに許されていません。以下を実行してからログインし直すか、sudoを付けて実行してください。

    sudo gpasswd -a $USER dialout

## 実行

開発はLinux上でおこなっていますが、おそらくMac/Windowsでも動作するはずです。すみません、まだバイナリのビルドをしていないので、Rustをインストールしてcargoで起動してください。

    cargo run

シリアルポートの選択になるので、M-6000Mが接続されたシリアルポートを選びます。

    Select serial port to connect:
    1) /dev/ttyUSB0
    2) /dev/ttyS0
    Enter number or 'q' to quit: 1

以下のようにJSONL形式で測定データが表示されます。

    {"raw":{"range":"Range0","digits":{"digits":[0,0,0,1]},"function":"Ohm","status":{"temperature_unit":"Celsius","sign":false,"is_battery_depleted":false,"is_overflow":false},"option2":{"is_dc":false,"is_ac":false,"is_auto":true}},"value":{"digits":"0.1","value_unit":{"prefix_unit":"None","base_unit":"Ohm"}}} 

## オプション

### シリアルポート指定

--portを指定することで、起動時のシリアルポート選択をスキップできます。

    cargo run -- --port /dev/ttyUSB0

### VOICEBOXサポート

[VOICEBOX](https://github.com/VOICEVOX/voicevox_core)による測定値の読み上げに対応しています。

[![VOICEBOXサポート](https://img.youtube.com/vi/gl671_m4UQ4/0.jpg)](https://www.youtube.com/watch?v=gl671_m4UQ4)

VOICEBOXはREST APIでアクセス可能になっている必要があります。簡単なのはDockerを使うことです。

    docker run -d -p '127.0.0.1:50021:50021' voicevox/voicevox_engine:cpu-ubuntu20.04-latest

以下のようにしてvoiceboxのURLを指定します。

    cargo run -- --voicebox-url http://localhost:50021

#### 話者の変更

--voicebox-speakerで変更できます。デフォルトは1です。

    cargo run -- --voicebox-url http://localhost:50021 --voicebox-speaker 9

#### 出力オーディオデバイスの指定

特に指定がなければデフォルトオーディオ出力デバイスが使用されます。変更したい場合、まず以下のようにして--audio-output-device-nameに存在しない名前を指定して起動します。

    cargo run -- --voicebox-url http://localhost:50021 --voicebox-speaker 9 --audio-output-device-name AAA
    
すると以下のようにデバイス名が一覧されるので、

    Unknown audio output device name: AAA
    8 audio output device detected:
      Device 'default'
        Configuration:
          Channels: 1
          Sample rate: 1
          Sample format: U8
        Configuration:
          Channels: 2
          Sample rate: 1

使用したいデバイス名を指定します。

    cargo run -- --voicebox-url http://localhost:50021 --voicebox-speaker 9 --audio-output-device-name pulse
