use std::sync::{Arc, Mutex};
use std::thread;

// Наш кольцевой буфер
#[derive(Debug)]
struct RingBuffer {
    data: Vec<Option<u8>>, // Хранилище данных
    head: usize,           // Указатель на начало (откуда читаем)
    tail: usize,           // Указатель на конец (куда пишем)
    size: usize,           // Текущее количество элементов
    capacity: usize,       // Максимальная вместимость
}

// Ошибки буфера
#[derive(Debug, PartialEq)]
enum BufferError {
    Full, // Буфер переполнен
}

impl RingBuffer {
    // Создаем новый буфер заданного размера
    fn new(capacity: usize) -> Self {
        RingBuffer {
            data: vec![None; capacity], // Заполняем None
            head: 0,
            tail: 0,
            size: 0,
            capacity,
        }
    }

    // Проверка на пустоту
    fn is_empty(&self) -> bool {
        self.size == 0
    }

    // Проверка на заполненность
    fn is_full(&self) -> bool {
        self.size == self.capacity
    }

    // Добавление элемента
    fn push(&mut self, value: u8) -> Result<(), BufferError> {
        if self.is_full() {
            return Err(BufferError::Full);
        }

        self.data[self.tail] = Some(value);
        self.tail = (self.tail + 1) % self.capacity; // Кольцевой буфер
        self.size += 1;
        Ok(())
    }

    // Извлечение элемента
    fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }

        let value = self.data[self.head].take();
        self.head = (self.head + 1) % self.capacity; // Кольцевой буфер
        self.size -= 1;
        value
    }
}

// Потокобезопасная обертка
#[derive(Debug)]
struct SafeRingBuffer {
    inner: Mutex<RingBuffer>, // Защищаем буфер мьютексом
}

impl SafeRingBuffer {
    fn new(capacity: usize) -> Self {
        SafeRingBuffer {
            inner: Mutex::new(RingBuffer::new(capacity)),
        }
    }

    // Потокобезопасное добавление
    fn push(&self, value: u8) -> Result<(), BufferError> {
        let mut buffer = self.inner.lock().unwrap(); // Блокируем доступ
        buffer.push(value)
        // Мьютекс автоматически разблокируется при выходе из области видимости
    }

    // Потокобезопасное извлечение
    fn pop(&self) -> Option<u8> {
        let mut buffer = self.inner.lock().unwrap(); // Блокируем доступ
        buffer.pop()
    }
}

fn main() {
    // Создаем потокобезопасный буфер на 5 элементов
    let buffer = Arc::new(SafeRingBuffer::new(5));

    // Демонстрация работы в одном потоке
    println!("=== Однопоточная демонстрация ===");
    buffer.push(10).unwrap();
    buffer.push(20).unwrap();
    println!("Извлекли: {:?}", buffer.pop()); // 10
    buffer.push(30).unwrap();
    println!("Извлекли: {:?}", buffer.pop()); // 20
    println!("Извлекли: {:?}", buffer.pop()); // 30

    // Многопоточная демонстрация
    println!("\n=== Многопоточная демонстрация ===");
    let buffer_clone = Arc::clone(&buffer);

    // Поток-писатель
    let writer = thread::spawn(move || {
        for i in 1..=5 {
            buffer_clone.push(i).unwrap();
            println!("Писатель записал: {}", i);
        }
    });

    // Поток-читатель
    let reader = thread::spawn(move || {
        for _ in 1..=5 {
            if let Some(val) = buffer.pop() {
                println!("Читатель прочитал: {}", val);
            }
        }
    });

    // Ждем завершения потоков
    writer.join().unwrap();
    reader.join().unwrap();
}

// Тесты
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_thread() {
        let buffer = SafeRingBuffer::new(3);

        // Заполняем буфер
        assert_eq!(buffer.push(1), Ok(()));
        assert_eq!(buffer.push(2), Ok(()));
        assert_eq!(buffer.push(3), Ok(()));
        assert_eq!(buffer.push(4), Err(BufferError::Full)); // Переполнение

        // Читаем данные
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.push(4), Ok(())); // Теперь можно записать
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None); // Буфер пуст
    }

    #[test]
    fn test_multi_thread() {
        let buffer = Arc::new(SafeRingBuffer::new(100));
        let mut handles = vec![];

        // Запускаем 5 писателей
        for i in 0..5 {
            let buffer = Arc::clone(&buffer);
            handles.push(thread::spawn(move || {
                for j in 1..=10 {
                    if let Err(e) = buffer.push(i * 20 + j) {
                        println!("Ошибка записи: {:?}", e);
                        break;
                    }
                }
            }));
        }

        // Ждем завершения писателей
        for handle in handles.drain(..) {
            handle.join().unwrap();
        }

        // Запускаем 5 читателей
        let results = Arc::new(Mutex::new(Vec::new()));
        for _ in 0..5 {
            let buffer = Arc::clone(&buffer);
            let results = Arc::clone(&results);
            handles.push(thread::spawn(move || {
                for _ in 1..=10 {
                    if let Some(val) = buffer.pop() {
                        results.lock().unwrap().push(val);
                    }
                }
            }));
        }

        // Ждем завершения читателей
        for handle in handles {
            handle.join().unwrap();
        }

        // Проверяем, что все данные прочитаны
        let results = results.lock().unwrap();
        assert_eq!(results.len(), 50);
    }
}
