use fmrs_core::piece::Color;
use serde::Deserialize;

/// HiddenMateで検討する協力系ルール。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MateRule {
    /// 攻方（黒）が受方玉を詰める。
    #[default]
    Helpmate,
    /// 受方（白）が攻方玉を詰める。
    HelpSelfmate,
}

/// 同じ駒台にある複数の覆面駒を着手時に個体識別できるか。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandVariableMode {
    /// V1、V2のように個体を選んで打つ現行ルール。
    #[default]
    Distinguishable,
    /// どの個体を打ったかは観測できず、可能な個体をすべて残す。
    Indistinguishable,
}

impl MateRule {
    pub(crate) fn terminal_turn(self) -> Color {
        match self {
            Self::Helpmate => Color::WHITE,
            Self::HelpSelfmate => Color::BLACK,
        }
    }

    pub(crate) fn initial_turn(self, plies: usize) -> Color {
        if plies % 2 == 0 {
            self.terminal_turn()
        } else {
            self.terminal_turn().opposite()
        }
    }
}
