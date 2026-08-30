#[contour]
fn route(request: u8) -> u8 {
    #[question("Есть <заявка> & она подходит?")]
    |&request| -> (accepted, rejected) { request > 0 };

    #[choice("Какой путь выбрать для этой заявки?")]
    #[case("Короткий путь")]
    #[case("Длинный путь с дополнительной проверкой")]
    |accepted, &request| -> (short, long) {
        match request {
            1 => (),
            _ => (),
        }
    };

    #[action("Подготовить короткий результат.")]
    |short, &request| -> short_value { request };

    #[action("Подготовить длинный результат, сохранив все важные детали заявки.")]
    |long, &request| -> long_value { request };

    #[merge]
    |short_value, long_value| -> selected {};

    #[action("Вернуть выбранный результат.")]
    |selected| -> result { selected };

    #[action("Отклонить заявку.")]
    |rejected, request| -> declined { request };
}
