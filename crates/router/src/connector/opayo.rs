mod transformers;

use std::fmt::Debug;

use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use transformers as opayo;

use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

use super::utils::RouterData;



#[derive(Debug, Clone)]
pub struct Opayo;

impl api::Payment for Opayo {}
impl api::PaymentSession for Opayo {}
impl api::ConnectorAccessToken for Opayo {}
impl api::PreVerify for Opayo {}
impl api::PaymentAuthorize for Opayo {}
impl api::PaymentSync for Opayo {}
impl api::PaymentCapture for Opayo {}
impl api::PaymentVoid for Opayo {}
impl api::Refund for Opayo {}
impl api::RefundExecute for Opayo {}
impl api::RefundSync for Opayo {}
impl api::PaymentToken for Opayo {}

#[async_trait::async_trait]
impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Opayo
{

    async fn execute_pretasks(
            &self,
            router_data: &mut types::TokenizationRouterData,
            app_state: &crate::routes::AppState,
        ) -> CustomResult<(), errors::ConnectorError> {
            println!("heyyyaa");
            let integ: Box<
                &(dyn ConnectorIntegration<
                    api::AuthorizeSessionToken,
                    types::AuthorizeSessionTokenData,
                    types::PaymentsResponseData,
                > + Send
                      + Sync
                      + 'static),
            > = Box::new(&Self);
            println!("heyyyaa working");
            let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::from((
                &router_data,
                types::AuthorizeSessionTokenData::from(&router_data),
            ));
            println!("heyloo working");
            let resp = services::execute_connector_processing_step(
                app_state,
                integ,
                authorize_data,
                payments::CallConnectorAction::Trigger,
            )
            .await?;
        println!("heyloo yess");
            router_data.session_token = resp.session_token;
            Ok(())
        }

    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string(),
        )];
        let mut auth = vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", req.get_session_token()?),
        )];
        header.append(&mut auth);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}card-identifiers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = opayo::OpayoCardIdentifierRequest::try_from(req)?;
        let opayo_req =
            utils::Encode::<opayo::OpayoCardIdentifierRequest>::encode_to_string_of_json(&connector_req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        println!("tokenreq->> {:?}", opayo_req);
        Ok(Some(opayo_req))
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                .body(types::TokenizationType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
        res: Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: opayo::OpayoCardIdentifierResponse = res
            .response
            .parse_struct("OpayoTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        println!("tokenrespoo->> {:?}", response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        api::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > for Opayo
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}merchant-session-keys",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = opayo::OpayoSessionRequest::try_from(req)?;
        let req =
        utils::Encode::<opayo::OpayoSessionRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        println!("mskreq->> {:?}", req);
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeSessionTokenRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeSessionTokenRouterData, errors::ConnectorError> {
        let response: opayo::OpayoSessionResponse =
            res.response.parse_struct("OpayoSessionResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            println!("mskrespoo->> {:?}", response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Opayo
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = self.get_auth_header(&req.connector_auth_type)?;
        header.push((
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string(),
        ));
        Ok(header)
    }
}

impl ConnectorCommon for Opayo {
    fn id(&self) -> &'static str {
        "opayo"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.opayo.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth = opayo::OpayoAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key =
            consts::BASE64_ENGINE.encode(format!("{}:{}", auth.api_key, auth.key1));
        Ok(vec![
            (headers::AUTHORIZATION.to_string(),format!("Basic {encoded_api_key}"))
        ])
        
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: opayo::OpayoErrorResponse =
            res.response
                .parse_struct("OpayoErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Opayo
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Opayo
{
}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Opayo
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Opayo
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}transactions", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = opayo::OpayoPaymentsRequest::try_from(req)?;
        let opayo_req =
            utils::Encode::<opayo::OpayoPaymentsRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(opayo_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: opayo::OpayoPaymentsResponse = res
            .response
            .parse_struct("Opayo PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Opayo
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    // https://pi-test.sagepay.com/api/v1/transactions/{transactionId}
    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let txn_id = req
        .request
        .connector_transaction_id
        .get_connector_transaction_id()
        .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!("{}transactions/{}", self.base_url(connectors), txn_id))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: opayo::OpayoPaymentsResponse = res
            .response
            .parse_struct("opayo PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Opayo
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: opayo::OpayoPaymentsResponse = res
            .response
            .parse_struct("Opayo PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Opayo
{

    // fn get_headers(
    //     &self,
    //     req: &types::PaymentsCancelRouterData,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
    //     self.build_headers(req, connectors)
    // }

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

    // fn get_url(
    //     &self,
    //     req: &types::PaymentsCancelRouterData,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<String, errors::ConnectorError> {
    //     let txn_id = req
    //     .request
    //     .connector_transaction_id
    //     .get_connector_transaction_id()
    //     .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
    //     Ok(format!("{}transactions/{}", self.base_url(connectors), txn_id))
    // }

    // fn get_request_body(
    //     &self,
    //     _req: &types::PaymentsCancelRouterData,
    // ) -> CustomResult<Option<String>, errors::ConnectorError> {
    //     Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    // }

    // fn build_request(
    //     &self,
    //     req: &types::PaymentsCancelRouterData,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
    //     Ok(Some(
    //         services::RequestBuilder::new()
    //             .method(services::Method::Post)
    //             .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
    //             .attach_default_headers()
    //             .headers(types::PaymentsVoidType::get_headers(
    //                 self, req, connectors,
    //             )?)
    //             .build(),
    //     ))
    // }

    // fn handle_response(
    //     &self,
    //     data: &types::PaymentsCancelRouterData,
    //     res: Response,
    // ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
    //     let response: opayo::OpayoPaymentsCancelResponse = res
    //         .response
    //         .parse_struct("Opayo PaymentsCaptureResponse")
    //         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    //     types::RouterData::try_from(types::ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    //     .change_context(errors::ConnectorError::ResponseHandlingFailed)
    // }

    // fn get_error_response(
    //     &self,
    //     res: Response,
    // ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res)
    // }

}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Opayo {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = opayo::OpayoRefundRequest::try_from(req)?;
        let opayo_req =
            utils::Encode::<opayo::OpayoRefundRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(opayo_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: opayo::RefundResponse = res
            .response
            .parse_struct("opayo RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Opayo {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: opayo::RefundResponse = res
            .response
            .parse_struct("opayo RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Opayo {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}