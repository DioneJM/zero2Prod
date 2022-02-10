use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn newsletter_form(
    flash_messages: IncomingFlashMessages
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for message in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", message.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Send Newsletter</title>
            </head>
            <body>
                {}
                <p>Create a newsletter:</p>
                <form action="/admin/newsletter" method="post">
                    <label>Title
                        <br/>
                        <input
                            type="text"
                            placeholder="Enter title of newsletter"
                            name="title"
                        >
                    </label>
                    <br>
                    <label>Text Content
                        <br/>
                        <textarea
                            rows=10
                            cols=60
                            placeholder="Newsletter content"
                            name="text_content"
                        ></textarea>
                    </label>
                    <br>
                    <label>HTML Content
                        <br/>
                        <textarea
                            rows=10
                            cols=60
                            placeholder="Newsletter content"
                            name="html_content"
                        ></textarea>
                    </label>
                    <br>
                    <button type="submit">Send newsletter</button>
                </form>
                <a href="/admin/dashboard">Dashboard</a>
            </body>
                </html>
            "#,
            msg_html
        )))
}