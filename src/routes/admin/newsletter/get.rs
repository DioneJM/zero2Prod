use actix_web::HttpResponse;

pub async fn newsletter_form() -> Result<HttpResponse, actix_web::Error> {
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
                <p>Create a newsletter:</p>
                <form action="/admin/newsletter" method="post">
                    <label>Title
                        <input
                            type="text"
                            placeholder="Enter title of newsletter"
                            name="title"
                        >
                    </label>
                    <br>
                    <label>Content
                        <input
                            type="text"
                            placeholder="Newsletter content"
                            name="content"
                        >
                    </label>
                    <br>
                    <button type="submit">Send newsletter</button>
                </form>
            </body>
            </html>
            "#
        )))
}