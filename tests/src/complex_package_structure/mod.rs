use self::proto::image::Image;
use self::proto::post::content::post_content_fragment;
use self::proto::post::content::PostContentFragment;
use self::proto::post::Post;
use self::proto::user::User;
use self::proto::Timestamp;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/complex_package_structure/__.rs"));
}

#[test]
fn test_complex_package_structure() {
    let user = User {
        id: "69a4cd96-b956-4fb1-9a97-b222eac33b8a".to_string(),
        name: "Test User".to_string(),
        created_at: Some(Timestamp {
            seconds: 1710366135,
            nanos: 0,
        }),
        ..User::default()
    };
    let posts = vec![
        Post::default(),
        Post {
            id: "aa1e751f-e287-4c6e-aa0f-f838f96a1a60".to_string(),
            author: Some(user),
            content: vec![
                PostContentFragment {
                    content: Some(post_content_fragment::Content::Text(
                        "Hello, world!".to_string(),
                    )),
                },
                PostContentFragment {
                    content: Some(post_content_fragment::Content::Image(Image {
                        name: "doggo.jpg".to_string(),
                        description: Some("A dog".to_string()),
                        data: vec![0, 1, 2, 3],
                    })),
                },
                PostContentFragment {
                    content: Some(post_content_fragment::Content::Link(
                        "https://example.com".to_string(),
                    )),
                },
            ],
            ..Post::default()
        },
        Post::default(),
    ];
    assert_eq!(posts.len(), 3);
    assert_eq!(posts[1].content.len(), 3);
    if let PostContentFragment {
        content: Some(post_content_fragment::Content::Image(Image { name, .. })),
    } = &posts[1].content[1]
    {
        assert_eq!(name, "doggo.jpg");
    } else {
        assert!(false, "Expected an image")
    }
}
