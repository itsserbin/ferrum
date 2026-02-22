use super::translations::Translations;

static UK: Translations = Translations {
    // --- Context menu ---
    menu_copy: "Копіювати",
    menu_paste: "Вставити",
    menu_select_all: "Вибрати все",
    menu_clear_selection: "Зняти виділення",
    menu_split_right: "Розділити праворуч",
    menu_split_down: "Розділити донизу",
    menu_split_left: "Розділити ліворуч",
    menu_split_up: "Розділити догори",
    menu_close_pane: "Закрити панель",
    menu_clear_terminal: "Очистити термінал",
    menu_reset_terminal: "Скинути термінал",
    menu_rename: "Перейменувати",
    menu_duplicate: "Дублювати",
    menu_close: "Закрити",

    // --- Close dialog ---
    close_dialog_title: "Закрити Ferrum?",
    close_dialog_body: "Закриття цього вікна термінала зупинить усі запущені процеси у вкладках.",
    close_dialog_confirm: "Закрити",
    close_dialog_cancel: "Скасувати",

    // --- Settings window ---
    settings_title: "Налаштування Ferrum",
    settings_tab_font: "Шрифт",
    settings_tab_theme: "Тема",
    settings_tab_terminal: "Термінал",
    settings_tab_layout: "Макет",
    settings_tab_security: "Безпека",
    settings_reset_to_defaults: "Скинути до стандартних",

    // --- Font tab ---
    font_size_label: "Розмір шрифту:",
    font_family_label: "Сімейство шрифтів:",
    font_line_padding_label: "Відступ рядка:",

    // --- Theme tab ---
    theme_label: "Тема:",

    // --- Terminal tab ---
    terminal_language_label: "Мова:",
    terminal_max_scrollback_label: "Макс. прокрутка:",
    terminal_cursor_blink_label: "Мерехтіння курсора (мс):",

    // --- Layout tab ---
    layout_window_padding_label: "Відступ вікна:",
    layout_pane_padding_label: "Відступ панелі:",
    layout_scrollbar_width_label: "Ширина смуги прокрутки:",
    layout_tab_bar_height_label: "Висота панелі вкладок:",

    // --- Security tab ---
    security_mode_label: "Режим безпеки:",
    security_mode_disabled: "Вимкнено",
    security_mode_standard: "Стандартний",
    security_mode_custom: "Власний",
    security_paste_protection_label: "Захист вставки",
    security_paste_protection_desc: "Попереджати перед вставкою тексту з підозрілими керуючими символами",
    security_block_title_query_label: "Блокувати запит заголовка",
    security_block_title_query_desc: "Забороняти програмам зчитувати заголовок вікна термінала",
    security_limit_cursor_jumps_label: "Обмежити переміщення курсора",
    security_limit_cursor_jumps_desc: "Обмежити відстань переміщення курсора керуючими послідовностями",
    security_clear_mouse_on_reset_label: "Очищати мишу при скиданні",
    security_clear_mouse_on_reset_desc: "Вимикати режими відстеження миші при скиданні термінала",

    // --- Security popup ---
    security_event_paste_newlines: "Виявлено вставку з новими рядками",
    security_event_title_query_blocked: "Заблоковано запит заголовка OSC/CSI",
    security_event_cursor_rewrite: "Виявлено перезапис курсора",
    security_event_mouse_leak_prevented: "Запобігнено витоку звітування миші",

    // --- macOS pin button ---
    macos_pin_window: "Закріпити вікно",
    macos_unpin_window: "Відкріпити вікно",
    macos_pin_tooltip: "Закріпити вікно поверх інших",
    macos_unpin_tooltip: "Відкріпити вікно",
    macos_settings: "Налаштування",

    // --- Update ---
    update_available: "Доступне оновлення {}",
    update_details: "Деталі",
    update_install: "Встановити",
    update_installing: "Встановлення…",
    settings_tab_updates: "Оновлення",
    update_current_version: "Поточна версія",
    update_check_now: "Перевірити оновлення",
    update_auto_check: "Автоперевірка оновлень",
};

pub fn translations() -> &'static Translations {
    &UK
}
