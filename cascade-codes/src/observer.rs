//! События кодека для журналирования.

use crate::DecodeStatus;

/// Событие восстановления данных или узла.
///
/// События приходят синхронно в порядке выполнения. При ошибке входа
/// (неверные длины, номера узлов) события не отправляются.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeEvent<'a> {
    /// Начато восстановление данных.
    DecodeStarted {
        /// Число переданных узлов.
        shares: usize,
        /// Число узлов, нужных для одной попытки (`k`).
        required: usize,
    },
    /// Сегмент восстановлен в ходе алгоритма 2.
    SegmentRecovered {
        /// Номер сегмента в порядке столбцов супер-матрицы.
        segment: usize,
        /// Режим сегмента.
        mode: usize,
    },
    /// Проверено подмножество из `k` узлов.
    SubsetTried {
        /// Узлы подмножества.
        nodes: &'a [usize],
        /// Сколько переданных узлов совпало с перекодированным результатом.
        agreeing: usize,
    },
    /// Восстановление данных завершено.
    DecodeFinished {
        /// Итоговый статус.
        status: DecodeStatus,
        /// Узлы, содержимое которых не совпало с результатом.
        suspect_nodes: &'a [usize],
    },
    /// Начато восстановление узла.
    RepairStarted {
        /// Отказавший узел.
        failed: usize,
        /// Помощники.
        helpers: &'a [usize],
    },
    /// Восстановлено пространство восстановления сегмента.
    RepairSegment {
        /// Номер сегмента.
        segment: usize,
        /// Режим сегмента.
        mode: usize,
        /// Символов от каждого помощника для этого сегмента.
        symbols_per_helper: usize,
    },
    /// Восстановление узла завершено.
    RepairFinished {
        /// Восстановленный узел.
        failed: usize,
        /// Всего символов получено от помощников.
        downloaded: usize,
    },
}

/// Получатель событий кодека.
pub trait CascadeObserver {
    /// Обрабатывает одно событие.
    fn on_event(&mut self, event: &CascadeEvent<'_>);
}

/// Наблюдатель, сохраняющий события в текстовом виде.
///
/// ```rust
/// use cascade_codes::{CascadeEvent, CascadeObserver, EventLog};
///
/// let mut log = EventLog::default();
/// log.on_event(&CascadeEvent::DecodeStarted { shares: 4, required: 3 });
/// assert_eq!(log.lines().len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventLog {
    lines: Vec<String>,
}

impl EventLog {
    /// Строки журнала в порядке событий.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

impl CascadeObserver for EventLog {
    fn on_event(&mut self, event: &CascadeEvent<'_>) {
        self.lines.push(format!("{event:?}"));
    }
}

pub(crate) fn emit(observer: &mut Option<&mut dyn CascadeObserver>, event: CascadeEvent<'_>) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&event);
    }
}

/// Повторное заимствование наблюдателя для вложенного вызова.
pub(crate) fn reborrow<'b>(
    observer: &'b mut Option<&mut dyn CascadeObserver>,
) -> Option<&'b mut dyn CascadeObserver> {
    match observer {
        Some(observer) => Some(&mut **observer),
        None => None,
    }
}
